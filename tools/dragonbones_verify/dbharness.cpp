// dbharness - headless DragonBonesCPP pose dumper.
//
// Usage: dbharness <ske.json> <tex.json> <out.json>
//
// Parses a DragonBones JSON skeleton + texture atlas JSON through the upstream
// DragonBonesCPP runtime (no renderer, no image loading), builds armature[0],
// and dumps bone world matrices and slot state/vertices for the setup pose and
// for every frame 0..frameCount (inclusive) of every animation.
//
// Coordinate space: DragonBones::yDown = true, i.e. everything stays in the
// JSON's native Y-down armature space (no flipping, no cocos-style negation).

#include "dragonBones/DragonBonesHeaders.h"
#include "rapidjson/document.h"
#include "rapidjson/error/en.h"

#include <cstdio>
#include <cstdlib>
#include <csignal>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>
#include <algorithm>

#ifdef _WIN32
#include <windows.h>
#include <crtdbg.h>
#endif

using namespace dragonBones;

// ---------------------------------------------------------------------------
// Crash / assert reporting: remember what we're doing so a crash can say where.
static std::string g_stage = "startup";

static void reportStage(const char* why)
{
    std::fprintf(stderr, "dbharness: FATAL (%s) during stage: %s\n", why, g_stage.c_str());
    std::fflush(stderr);
}

static void onAbort(int)
{
    reportStage("abort/assert");
    std::_Exit(3);
}

#ifdef _WIN32
static LONG WINAPI onSEH(EXCEPTION_POINTERS* ep)
{
    char buf[128];
    std::snprintf(buf, sizeof(buf), "SEH exception 0x%08lX at %p",
        (unsigned long)ep->ExceptionRecord->ExceptionCode, ep->ExceptionRecord->ExceptionAddress);
    reportStage(buf);
    std::_Exit(4);
    return EXCEPTION_EXECUTE_HANDLER;
}
#endif

// ---------------------------------------------------------------------------
// Minimal runtime glue.

class HProxy : public IArmatureProxy
{
public:
    Armature* armature = nullptr;
    bool hasDBEventListener(const std::string&) const override { return false; }
    void dispatchDBEvent(const std::string&, EventObject*) override {}
    void addDBEventListener(const std::string&, const std::function<void(EventObject*)>&) override {}
    void removeDBEventListener(const std::string&, const std::function<void(EventObject*)>&) override {}
    void dbInit(Armature* a) override { armature = a; }
    void dbClear() override { armature = nullptr; }
    void dbUpdate() override {}
    void dispose(bool) override {}
    Armature* getArmature() const override { return armature; }
    Animation* getAnimation() const override { return armature ? armature->getAnimation() : nullptr; }
};

class HTextureData : public TextureData
{
    BIND_CLASS_TYPE_B(HTextureData);
public:
    HTextureData() { _onClear(); }
    virtual ~HTextureData() { _onClear(); }
};

class HTextureAtlasData : public TextureAtlasData
{
    BIND_CLASS_TYPE_B(HTextureAtlasData);
public:
    HTextureAtlasData() { _onClear(); }
    virtual ~HTextureAtlasData() { _onClear(); }
    TextureData* createTexture() const override { return BaseObject::borrowObject<HTextureData>(); }
};

// Slot with no-op render hooks. Exposes protected state for dumping.
class HSlot : public Slot
{
    BIND_CLASS_TYPE_A(HSlot);
protected:
    void _initDisplay(void*, bool) override {}
    void _disposeDisplay(void*, bool) override {}
    void _onUpdateDisplay() override {}
    void _addDisplay() override {}
    void _replaceDisplay(void*, bool) override {}
    void _removeDisplay() override {}
    void _updateZOrder() override {}
    void _updateFrame() override {}
    void _updateMesh() override {}
    void _updateTransform() override {}
    void _identityTransform() override {}
public:
    void _updateVisible() override {}
    void _updateBlendMode() override {}
    void _updateColor() override {}

    TextureData* textureData() const { return _textureData; }
    void* displayPtr() const { return _display; }
    Armature* childArmature() const { return _childArmature; }
};

// Dummy non-null display objects so BaseFactory::_getSlotDisplay puts
// something in the slot display list (it uses slot->_rawDisplay / _meshDisplay).
static char g_rawDisplayTag;
static char g_meshDisplayTag;

class HFactory : public BaseFactory
{
public:
    HProxy eventManager;
    HFactory()
    {
        _dragonBones = new DragonBones(&eventManager);
    }
protected:
    TextureAtlasData* _buildTextureAtlasData(TextureAtlasData* textureAtlasData, void*) const override
    {
        if (textureAtlasData == nullptr)
            textureAtlasData = BaseObject::borrowObject<HTextureAtlasData>();
        return textureAtlasData;
    }
    Armature* _buildArmature(const BuildArmaturePackage& dataPackage) const override
    {
        const auto armature = BaseObject::borrowObject<Armature>();
        const auto proxy = new HProxy(); // leaked on purpose (process-lifetime)
        armature->init(dataPackage.armature, proxy, proxy, _dragonBones);
        return armature;
    }
    Slot* _buildSlot(const BuildArmaturePackage&, const SlotData* slotData, Armature* armature) const override
    {
        const auto slot = BaseObject::borrowObject<HSlot>();
        slot->init(slotData, armature, &g_rawDisplayTag, &g_meshDisplayTag);
        return slot;
    }
};

// ---------------------------------------------------------------------------
// JSON writing helpers.

static std::string jsonEscape(const std::string& s)
{
    std::string o;
    o.reserve(s.size() + 2);
    o += '"';
    for (unsigned char c : s)
    {
        switch (c)
        {
        case '"': o += "\\\""; break;
        case '\\': o += "\\\\"; break;
        case '\n': o += "\\n"; break;
        case '\r': o += "\\r"; break;
        case '\t': o += "\\t"; break;
        default:
            if (c < 0x20) { char b[8]; std::snprintf(b, sizeof(b), "\\u%04x", c); o += b; }
            else o += (char)c;
        }
    }
    o += '"';
    return o;
}

static void num(std::string& o, double v)
{
    if (!(v == v) || v > 1e30 || v < -1e30) { o += "null"; return; } // NaN / inf guard
    char b[32];
    std::snprintf(b, sizeof(b), "%.9g", v);
    o += b;
}

// ---------------------------------------------------------------------------
// Vertex computation.

// Mesh: mirrors CCSlot::_updateMesh / _updateFrame + _updateTransform, minus the
// cocos Y negation.
//  - weighted:   v = sum_j weight_j * (boneGlobal_j * (local_j*scale + ffd))
//                (the slot display gets identity transform, so these ARE armature space)
//  - unweighted: v = slotGlobal * (local*scale + ffd); _pivotX/_pivotY are 0 for meshes
//                (Slot::_updateDisplayData sets them to 0 when verticesData != null).
static void meshVerts(HSlot* slot, std::vector<float>& out)
{
    const auto dv = slot->_deformVertices;
    const auto verticesData = dv->verticesData;
    const auto data = verticesData->data;
    const auto intArray = data->intArray;
    const auto floatArray = data->floatArray;
    const auto scale = slot->getArmature()->_armatureData->scale;
    const auto& ffd = dv->vertices;
    const bool hasFFD = !ffd.empty();
    const auto vertexCount = (std::size_t)intArray[verticesData->offset + (unsigned)BinaryOffset::MeshVertexCount];
    const auto weightData = verticesData->weight;

    if (weightData != nullptr)
    {
        const auto& bones = dv->bones;
        int weightFloatOffset = intArray[weightData->offset + (unsigned)BinaryOffset::WeigthFloatOffset];
        if (weightFloatOffset < 0) weightFloatOffset += 65536;

        for (std::size_t i = 0, iB = weightData->offset + (unsigned)BinaryOffset::WeigthBoneIndices + bones.size(),
             iV = (std::size_t)weightFloatOffset, iF = 0; i < vertexCount; ++i)
        {
            const auto boneCount = (std::size_t)intArray[iB++];
            float xG = 0.0f, yG = 0.0f;
            for (std::size_t j = 0; j < boneCount; ++j)
            {
                const auto boneIndex = (unsigned)intArray[iB++];
                const auto bone = bones[boneIndex];
                // NOTE: CCSlot only advances iV/iF when bone != nullptr (which would
                // desync); we always consume the entry so a missing bone just drops its weight.
                const auto weight = floatArray[iV++];
                float xL = floatArray[iV++] * scale;
                float yL = floatArray[iV++] * scale;
                if (hasFFD) { xL += ffd[iF++]; yL += ffd[iF++]; }
                if (bone != nullptr)
                {
                    const auto& m = bone->globalTransformMatrix;
                    xG += (m.a * xL + m.c * yL + m.tx) * weight;
                    yG += (m.b * xL + m.d * yL + m.ty) * weight;
                }
            }
            out.push_back(xG);
            out.push_back(yG);
        }
    }
    else
    {
        int vertexOffset = intArray[verticesData->offset + (unsigned)BinaryOffset::MeshFloatOffset];
        if (vertexOffset < 0) vertexOffset += 65536;
        const auto& m = slot->globalTransformMatrix;
        for (std::size_t i = 0, l = vertexCount * 2; i < l; i += 2)
        {
            float x = floatArray[vertexOffset + i] * scale;
            float y = floatArray[vertexOffset + i + 1] * scale;
            if (hasFFD && i + 1 < ffd.size()) { x += ffd[i]; y += ffd[i + 1]; }
            x -= slot->_pivotX; // always 0 for meshes, kept for symmetry with _updateTransform
            y -= slot->_pivotY;
            out.push_back(m.a * x + m.c * y + m.tx);
            out.push_back(m.b * x + m.d * y + m.ty);
        }
    }
}

// Image corners.
//
// Derivation (yDown = true):
//  Slot::_updateDisplayData computes, in armature units (scale = atlas.scale * armature.scale),
//     _pivotX = pivot.x * W * scale + frame.x * scale
//     _pivotY = pivot.y * H * scale + frame.y * scale
//  where W,H = frame size if trimmed, else region size (swapped when rotated), and
//  pivot defaults to (0.5,0.5) (the display's registration point is the image centre).
//  frame.x/frame.y are the (<= 0) atlas frameX/frameY, so _pivot is the registration
//  point measured from the TOP-LEFT OF THE PACKED REGION, in Y-down local pixels.
//  (For yDown=false the runtime instead sets _pivotY = h*scale - _pivotY; not used here.)
//
//  A y-down renderer (SFMLSlot::_updateTransform; CCSlot is the same with the Y flip)
//  places the sprite with linear part = globalTransformMatrix * textureScale and origin
//     pos = (tx,ty) - M_lin * (_pivotX,_pivotY),
//  so texel p (0..w, 0..h in region pixels) lands at M * (p*scale - pivot).
//  Hence the corners are slot->globalTransformMatrix * q for
//     q in [(-pX,-pY), (w-pX,-pY), (w-pX,h-pY), (-pX,h-pY)]
//  with w,h = the region's displayed (unrotated) size * scale, i.e. region.height/width
//  swapped when rotated.
static void imageVerts(HSlot* slot, TextureData* tex, std::vector<float>& out)
{
    const float scale = tex->parent->scale * slot->getArmature()->_armatureData->scale;
    float w = tex->rotated ? tex->region.height : tex->region.width;
    float h = tex->rotated ? tex->region.width : tex->region.height;
    w *= scale;
    h *= scale;
    const float pX = slot->_pivotX, pY = slot->_pivotY;
    const float q[4][2] = { { -pX, -pY }, { w - pX, -pY }, { w - pX, h - pY }, { -pX, h - pY } };
    const auto& m = slot->globalTransformMatrix;
    for (int i = 0; i < 4; ++i)
    {
        out.push_back(m.a * q[i][0] + m.c * q[i][1] + m.tx);
        out.push_back(m.b * q[i][0] + m.d * q[i][1] + m.ty);
    }
}

static void slotVerts(HSlot* slot, std::vector<float>& out)
{
    out.clear();
    if (slot->getDisplayIndex() < 0 || slot->displayPtr() == nullptr) return;
    if (slot->childArmature() != nullptr) return;          // child armatures not dumped
    if (!slot->getParent()->getVisible()) return;          // hidden bone => hidden slot
    const auto dd = slot->_displayData;
    if (dd == nullptr) return;
    auto tex = slot->textureData();

    const bool isMesh = slot->displayPtr() == &g_meshDisplayTag &&
        slot->_deformVertices != nullptr && slot->_deformVertices->verticesData != nullptr;
    if (isMesh)
    {
        if (tex == nullptr) return; // renderer draws nothing without a texture
        meshVerts(slot, out);
        return;
    }
    if (dd->type == DisplayType::Image && tex != nullptr)
    {
        imageVerts(slot, tex, out);
    }
}

// ---------------------------------------------------------------------------

static void writeFrame(std::string& o, Armature* armature)
{
    o += "{\"bones\":{";
    bool first = true;
    for (const auto bone : armature->getBones())
    {
        if (!first) o += ',';
        first = false;
        o += jsonEscape(bone->getName());
        o += ":[";
        const auto& m = bone->globalTransformMatrix;
        num(o, m.a); o += ','; num(o, m.b); o += ','; num(o, m.c); o += ',';
        num(o, m.d); o += ','; num(o, m.tx); o += ','; num(o, m.ty);
        o += ']';
    }
    o += "},\"slots\":{";

    std::vector<Slot*> order(armature->getSlots().begin(), armature->getSlots().end());
    std::stable_sort(order.begin(), order.end(), [](Slot* a, Slot* b) { return a->_zOrder < b->_zOrder; });

    std::vector<float> verts;
    first = true;
    for (std::size_t di = 0; di < order.size(); ++di)
    {
        const auto slot = static_cast<HSlot*>(order[di]);
        if (!first) o += ',';
        first = false;
        o += jsonEscape(slot->getName());
        o += ":{\"displayIndex\":";
        o += std::to_string(slot->getDisplayIndex());
        o += ",\"drawOrder\":";
        o += std::to_string(di);
        const auto& c = slot->_colorTransform;
        o += ",\"color\":[";
        num(o, c.alphaMultiplier); o += ','; num(o, c.redMultiplier); o += ',';
        num(o, c.greenMultiplier); o += ','; num(o, c.blueMultiplier);
        o += "],\"verts\":[";
        slotVerts(slot, verts);
        for (std::size_t i = 0; i < verts.size(); ++i)
        {
            if (i) o += ',';
            num(o, verts[i]);
        }
        o += "]}";
    }
    o += "}}";
}

static bool readFile(const char* path, std::string& out)
{
    std::ifstream f(path, std::ios::binary);
    if (!f) return false;
    std::stringstream ss;
    ss << f.rdbuf();
    out = ss.str();
    return true;
}

static bool checkJson(const char* label, const std::string& text)
{
    rapidjson::Document d;
    d.Parse(text.c_str());
    if (d.HasParseError())
    {
        std::fprintf(stderr, "dbharness: %s is not valid JSON: %s (offset %u)\n", label,
            rapidjson::GetParseError_En(d.GetParseError()), (unsigned)d.GetErrorOffset());
        return false;
    }
    return true;
}

int main(int argc, char** argv)
{
#ifdef _WIN32
    _set_error_mode(_OUT_TO_STDERR);
    _set_abort_behavior(0, _WRITE_ABORT_MSG | _CALL_REPORTFAULT);
    _CrtSetReportMode(_CRT_ASSERT, _CRTDBG_MODE_FILE);
    _CrtSetReportFile(_CRT_ASSERT, _CRTDBG_FILE_STDERR);
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    SetUnhandledExceptionFilter(onSEH);
#endif
    std::signal(SIGABRT, onAbort);

    if (argc != 4)
    {
        std::fprintf(stderr, "usage: dbharness <ske.json> <tex.json> <out.json>\n");
        return 2;
    }

    DragonBones::yDown = true; // stay in the JSON's native Y-down space

    std::string ske, tex;
    if (!readFile(argv[1], ske)) { std::fprintf(stderr, "dbharness: cannot read %s\n", argv[1]); return 1; }
    if (!readFile(argv[2], tex)) { std::fprintf(stderr, "dbharness: cannot read %s\n", argv[2]); return 1; }
    if (!checkJson("ske", ske) || !checkJson("tex", tex)) return 1;

    HFactory factory;

    g_stage = "parseDragonBonesData";
    const auto dbData = factory.parseDragonBonesData(ske.c_str());
    if (dbData == nullptr) { std::fprintf(stderr, "dbharness: parseDragonBonesData returned null (bad version/format?)\n"); return 1; }
    if (dbData->armatureNames.empty()) { std::fprintf(stderr, "dbharness: no armatures in skeleton\n"); return 1; }

    g_stage = "parseTextureAtlasData";
    // Register the atlas under the skeleton's name: BaseFactory looks textures up by
    // the DragonBonesData name. Warn if the atlas file's own "name" disagrees.
    const auto atlas = factory.parseTextureAtlasData(tex.c_str(), nullptr, dbData->name);
    if (atlas == nullptr) { std::fprintf(stderr, "dbharness: parseTextureAtlasData failed\n"); return 1; }
    if (atlas->name != dbData->name)
        std::fprintf(stderr, "dbharness: WARNING atlas name \"%s\" != skeleton name \"%s\" (runtimes loading by file name may miss textures)\n",
            atlas->name.c_str(), dbData->name.c_str());

    const auto armatureName = dbData->armatureNames[0];
    g_stage = "buildArmature(" + armatureName + ")";
    const auto armature = factory.buildArmature(armatureName, dbData->name);
    if (armature == nullptr) { std::fprintf(stderr, "dbharness: buildArmature failed\n"); return 1; }

    {
        const auto ad = armature->getArmatureData();
        if (armature->getBones().size() != ad->bones.size() || armature->getSlots().size() != ad->slots.size())
            std::fprintf(stderr, "dbharness: WARNING built %zu bones / %zu slots but data has %zu / %zu (bad parent refs?)\n",
                armature->getBones().size(), armature->getSlots().size(), ad->bones.size(), ad->slots.size());
    }

    // Warn about slots whose displays reference textures that aren't in the atlas.
    for (const auto slot : armature->getSlots())
    {
        const auto dds = slot->getRawDisplayDatas();
        if (!dds) continue;
        for (const auto dd : *dds)
        {
            if (!dd) continue;
            TextureData* t = nullptr;
            if (dd->type == DisplayType::Image) t = static_cast<ImageDisplayData*>(dd)->texture;
            else if (dd->type == DisplayType::Mesh) t = static_cast<MeshDisplayData*>(dd)->texture;
            else continue;
            if (t == nullptr)
                std::fprintf(stderr, "dbharness: WARNING slot \"%s\" display \"%s\" has no texture in atlas\n",
                    slot->getName().c_str(), dd->path.c_str());
        }
    }

    std::string o;
    o.reserve(1 << 20);

    g_stage = "setup pose advanceTime(0)";
    armature->advanceTime(0.0f);
    o += "{\"armature\":";
    o += jsonEscape(armatureName);
    o += ",\"setup\":";
    writeFrame(o, armature);

    o += ",\"animations\":{";
    const auto animation = armature->getAnimation();
    const auto names = animation->getAnimationNames(); // copy
    bool firstAnim = true;
    for (const auto& name : names)
    {
        const auto animData = mapFind(animation->getAnimations(), name);
        if (animData == nullptr) continue;
        if (!firstAnim) o += ',';
        firstAnim = false;
        o += jsonEscape(name);
        o += ":{\"duration\":";
        o += std::to_string(animData->frameCount);
        o += ",\"frameRate\":";
        o += std::to_string(armature->_armatureData->frameRate);
        o += ",\"frames\":[";
        for (unsigned f = 0; f <= animData->frameCount; ++f)
        {
            g_stage = "animation \"" + name + "\" frame " + std::to_string(f);
            // reset() drops any previous AnimationState so nothing fades/blends in from
            // the prior sample; gotoAndStopByFrame uses resetToPose=true so bones/slots
            // without timelines get pose timelines back to setup. A stopped state still
            // evaluates its timelines at _time on advanceTime(0).
            animation->reset();
            // sample just after frame f: f / frameRate can round to a hair before the frame,
            // which shows the previous frame of stepped timelines (display index, held keys).
            // The last frame is the animation's end, so sample it exactly.
            const auto frameRate = (float)armature->_armatureData->frameRate;
            const auto state = f < animData->frameCount
                ? animation->gotoAndStopByTime(name, ((float)f + 0.001f) / frameRate)
                : animation->gotoAndStopByFrame(name, f);
            if (state == nullptr)
            {
                std::fprintf(stderr, "dbharness: gotoAndStopByFrame(%s,%u) returned null\n", name.c_str(), f);
                return 1;
            }
            armature->advanceTime(0.0f);
            if (f) o += ',';
            writeFrame(o, armature);
        }
        o += "]}";
    }
    o += "}}\n";

    g_stage = "writing output";
    std::ofstream out(argv[3], std::ios::binary);
    if (!out) { std::fprintf(stderr, "dbharness: cannot write %s\n", argv[3]); return 1; }
    out << o;
    out.close();

    std::fprintf(stderr, "dbharness: ok armature=%s bones=%zu slots=%zu animations=%zu\n",
        armatureName.c_str(), armature->getBones().size(), armature->getSlots().size(), names.size());
    return 0;
}
