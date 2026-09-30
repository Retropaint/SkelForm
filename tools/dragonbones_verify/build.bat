@echo off
rem Builds dbharness.exe (upstream DragonBonesCPP core + headless harness) with MSVC.
rem
rem usage: build.bat [path\to\DragonBonesCPP]
rem   The DragonBonesCPP checkout can also be given via the DRAGONBONES_CPP env var.
rem   Output goes to tools\dragonbones_verify\build\ (git-ignored).
setlocal
set "HERE=%~dp0"
set "DB=%~1"
if "%DB%"=="" set "DB=%DRAGONBONES_CPP%"
if "%DB%"=="" (
    echo error: pass the DragonBonesCPP checkout path, or set DRAGONBONES_CPP
    echo        git clone --depth 1 https://github.com/DragonBones/DragonBonesCPP.git
    exit /b 2
)
if not exist "%DB%\DragonBones\src\dragonBones\DragonBonesHeaders.h" (
    echo error: "%DB%" does not look like a DragonBonesCPP checkout
    exit /b 2
)

set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
set "VSDIR="
for /f "usebackq tokens=*" %%i in (`call "%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSDIR=%%i"
if "%VSDIR%"=="" (
    echo error: no Visual Studio install with the C++ toolset found
    exit /b 2
)
call "%VSDIR%\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 exit /b 1

set "OUT=%HERE%build"
if not exist "%OUT%\obj" mkdir "%OUT%\obj"
set SRCS=
for /r "%DB%\DragonBones\src\dragonBones" %%f in (*.cpp) do call set SRCS=%%SRCS%% "%%f"
cl /nologo /std:c++14 /EHsc /O2 /Zi /MT /W1 /utf-8 /DNOMINMAX /D_CRT_SECURE_NO_WARNINGS ^
   /I"%DB%\DragonBones\src" /I"%DB%\3rdParty" ^
   /Fo"%OUT%\obj\\" /Fd"%OUT%\obj\vc.pdb" /Fe"%OUT%\dbharness.exe" ^
   "%HERE%dbharness.cpp" %SRCS% /link /DEBUG
exit /b %ERRORLEVEL%
