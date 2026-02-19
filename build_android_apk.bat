@echo off
setlocal EnableDelayedExpansion

cd /d "%~dp0"

echo ========================================
echo   In Falsus Editor - Android APK Build
echo ========================================
echo.

if not exist "sign\rotaeno.jks" (
    echo [error] sign\rotaeno.jks not found
    exit /b 1
)

if "%ANDROID_HOME%"=="" (
    if not "%ANDROID_SDK_ROOT%"=="" (
        set "ANDROID_HOME=%ANDROID_SDK_ROOT%"
    ) else (
        set "ANDROID_HOME=%LOCALAPPDATA%\Android\Sdk"
    )
)
if not exist "%ANDROID_HOME%\ndk" (
    echo [error] Android NDK folder not found under "%ANDROID_HOME%\ndk"
    exit /b 1
)
if not exist "%ANDROID_HOME%\build-tools" (
    echo [error] Android build-tools folder not found under "%ANDROID_HOME%\build-tools"
    exit /b 1
)
if not exist "%ANDROID_HOME%\platforms" (
    echo [error] Android platforms folder not found under "%ANDROID_HOME%\platforms"
    exit /b 1
)
if "%ANDROID_NDK_HOME%"=="" (
    for /f "delims=" %%D in ('dir /b /ad "%ANDROID_HOME%\ndk" ^| sort /r') do (
        set "ANDROID_NDK_HOME=%ANDROID_HOME%\ndk\%%D"
        goto :ndk_found
    )
)
:ndk_found
if not exist "%ANDROID_NDK_HOME%\toolchains\llvm\prebuilt\windows-x86_64\bin\clang.exe" (
    echo [error] invalid ANDROID_NDK_HOME: "%ANDROID_NDK_HOME%"
    exit /b 1
)
echo [env] ANDROID_NDK_HOME=%ANDROID_NDK_HOME%

set "APKSIGNER_EXE="
set "ZIPALIGN_EXE="
set "AAPT_EXE="
set "D8_EXE="
for /f "delims=" %%V in ('dir /b /ad "%ANDROID_HOME%\build-tools" ^| sort /r') do (
    if exist "%ANDROID_HOME%\build-tools\%%V\apksigner.bat" (
        if exist "%ANDROID_HOME%\build-tools\%%V\zipalign.exe" (
            if exist "%ANDROID_HOME%\build-tools\%%V\aapt.exe" (
                if exist "%ANDROID_HOME%\build-tools\%%V\d8.bat" (
                    set "APKSIGNER_EXE=%ANDROID_HOME%\build-tools\%%V\apksigner.bat"
                    set "ZIPALIGN_EXE=%ANDROID_HOME%\build-tools\%%V\zipalign.exe"
                    set "AAPT_EXE=%ANDROID_HOME%\build-tools\%%V\aapt.exe"
                    set "D8_EXE=%ANDROID_HOME%\build-tools\%%V\d8.bat"
                    goto :build_tools_found
                )
            )
        )
    )
)
:build_tools_found
if "%APKSIGNER_EXE%"=="" (
    echo [error] no usable apksigner.bat found under Android build-tools
    exit /b 1
)
if "%ZIPALIGN_EXE%"=="" (
    echo [error] no usable zipalign.exe found under Android build-tools
    exit /b 1
)
if "%AAPT_EXE%"=="" (
    echo [error] no usable aapt.exe found under Android build-tools
    exit /b 1
)
if "%D8_EXE%"=="" (
    echo [error] no usable d8.bat found under Android build-tools
    exit /b 1
)
echo [env] APKSIGNER=%APKSIGNER_EXE%
echo [env] ZIPALIGN=%ZIPALIGN_EXE%
echo [env] AAPT=%AAPT_EXE%
echo [env] D8=%D8_EXE%

set "ANDROID_JAR="
for /f "delims=" %%V in ('dir /b /ad "%ANDROID_HOME%\platforms\android-*" ^| sort /r') do (
    if exist "%ANDROID_HOME%\platforms\%%V\android.jar" (
        set "ANDROID_JAR=%ANDROID_HOME%\platforms\%%V\android.jar"
        goto :android_jar_found
    )
)
:android_jar_found
if "%ANDROID_JAR%"=="" (
    echo [error] no android.jar found under Android platforms
    exit /b 1
)
echo [env] ANDROID_JAR=%ANDROID_JAR%

set "JAVAC_EXE="
if not "%JAVA_HOME%"=="" (
    if exist "%JAVA_HOME%\bin\javac.exe" set "JAVAC_EXE=%JAVA_HOME%\bin\javac.exe"
)
if "%JAVAC_EXE%"=="" (
    if exist "%ProgramFiles%\Android\Android Studio\jbr\bin\javac.exe" (
        set "JAVAC_EXE=%ProgramFiles%\Android\Android Studio\jbr\bin\javac.exe"
    )
)
if "%JAVAC_EXE%"=="" (
    for /f "delims=" %%J in ('where javac 2^>nul') do (
        set "JAVAC_EXE=%%J"
        goto :javac_found
    )
)
:javac_found
if "%JAVAC_EXE%"=="" (
    echo [error] javac.exe not found. Set JAVA_HOME or install Android Studio.
    exit /b 1
)
echo [env] JAVAC=%JAVAC_EXE%

set "JAR_EXE="
for %%P in ("%JAVAC_EXE%") do (
    if exist "%%~dpPjar.exe" set "JAR_EXE=%%~dpPjar.exe"
)
if "%JAR_EXE%"=="" (
    if not "%JAVA_HOME%"=="" (
        if exist "%JAVA_HOME%\bin\jar.exe" set "JAR_EXE=%JAVA_HOME%\bin\jar.exe"
    )
)
if "%JAR_EXE%"=="" (
    for /f "delims=" %%J in ('where jar 2^>nul') do (
        set "JAR_EXE=%%J"
        goto :jar_found
    )
)
:jar_found
if "%JAR_EXE%"=="" (
    echo [error] jar.exe not found. Set JAVA_HOME or install Android Studio.
    exit /b 1
)
echo [env] JAR=%JAR_EXE%

set "MINIQUAD_JAVA_DIR="
for /d %%R in ("%USERPROFILE%\.cargo\registry\src\index.crates.io-*") do (
    for /f "delims=" %%D in ('dir /b /ad "%%R\miniquad-*" ^| sort /r') do (
        if exist "%%R\%%D\java\MainActivity.java" (
            if exist "%%R\%%D\java\QuadNative.java" (
                set "MINIQUAD_JAVA_DIR=%%R\%%D\java"
                goto :miniquad_java_found
            )
        )
    )
)
:miniquad_java_found
if "%MINIQUAD_JAVA_DIR%"=="" (
    echo [error] miniquad java templates not found in Cargo registry
    exit /b 1
)
echo [env] MINIQUAD_JAVA_DIR=%MINIQUAD_JAVA_DIR%

set "APP_PACKAGE=rust.in_falsus_editor"
set "APP_PACKAGE_PATH=rust\in_falsus_editor"

set "CARGO_APK_RELEASE_KEYSTORE=%CD%\sign\rotaeno.jks"
set "CARGO_APK_RELEASE_KEYSTORE_PASSWORD=rotaeno"

if not exist "projects\alamode\alamode.spc" (
    echo [error] projects\alamode\alamode.spc not found
    exit /b 1
)
if not exist "projects\alamode\alamode.iffproj" (
    echo [error] projects\alamode\alamode.iffproj not found
    exit /b 1
)
if not exist "projects\alamode\music.ogg" (
    echo [error] projects\alamode\music.ogg not found
    exit /b 1
)
if not exist "assets\tap.wav" (
    echo [error] assets\tap.wav not found
    exit /b 1
)
if not exist "assets\arc.wav" (
    echo [error] assets\arc.wav not found
    exit /b 1
)

set "ANDROID_CJK_FONT_SRC="
for %%F in (
    "assets\cjk_font.ttf"
    "assets\HarmonyOS_Sans_SC_Regular.ttf"
    "assets\HarmonyOS_Sans_Regular.ttf"
    "assets\simhei.ttf"
    "C:\Windows\Fonts\NotoSansSC-VF.ttf"
    "C:\Windows\Fonts\MiSans-Regular.otf"
    "C:\Windows\Fonts\simhei.ttf"
    "C:\Windows\Fonts\simfang.ttf"
    "C:\Windows\Fonts\simsunb.ttf"
) do (
    if exist "%%~F" (
        set "ANDROID_CJK_FONT_SRC=%%~F"
        goto :font_src_found
    )
)
:font_src_found
if "%ANDROID_CJK_FONT_SRC%"=="" (
    echo [error] no CJK font source found. Add one in assets\ or install a Chinese font in C:\Windows\Fonts.
    exit /b 1
)
echo [env] CJK_FONT_SRC=%ANDROID_CJK_FONT_SRC%

echo [1/7] Preparing android_assets ...
if exist "android_assets" rmdir /s /q "android_assets"
mkdir "android_assets\assets" >nul
mkdir "android_assets\projects\alamode" >nul
copy /Y "assets\tap.wav" "android_assets\assets\tap.wav" >nul
copy /Y "assets\arc.wav" "android_assets\assets\arc.wav" >nul
copy /Y "%ANDROID_CJK_FONT_SRC%" "android_assets\assets\cjk_font.ttf" >nul
copy /Y "projects\alamode\alamode.spc" "android_assets\projects\alamode\alamode.spc" >nul
copy /Y "projects\alamode\alamode.iffproj" "android_assets\projects\alamode\alamode.iffproj" >nul
copy /Y "projects\alamode\music.ogg" "android_assets\projects\alamode\music.ogg" >nul

echo [2/7] Building classes.dex for miniquad MainActivity ...
set "JAVA_WORK_DIR=build\android_java"
if exist "%JAVA_WORK_DIR%" rmdir /s /q "%JAVA_WORK_DIR%"
mkdir "%JAVA_WORK_DIR%\src\%APP_PACKAGE_PATH%" >nul
mkdir "%JAVA_WORK_DIR%\src\quad_native" >nul
mkdir "%JAVA_WORK_DIR%\classes" >nul
mkdir "%JAVA_WORK_DIR%\dex" >nul

powershell -NoProfile -Command "(Get-Content -Raw '%MINIQUAD_JAVA_DIR%\MainActivity.java').Replace('TARGET_PACKAGE_NAME','%APP_PACKAGE%').Replace('LIBRARY_NAME','in_falsus_editor') | Set-Content -NoNewline '%JAVA_WORK_DIR%\src\%APP_PACKAGE_PATH%\MainActivity.java'"
if errorlevel 1 (
    echo [error] failed to prepare MainActivity.java
    exit /b 1
)
copy /Y "%MINIQUAD_JAVA_DIR%\QuadNative.java" "%JAVA_WORK_DIR%\src\quad_native\QuadNative.java" >nul

powershell -NoProfile -Command "(Get-Content -Raw '%JAVA_WORK_DIR%\src\%APP_PACKAGE_PATH%\MainActivity.java').Replace('QuadNative.activityOnCreate(this);','QuadNative.initAndroidContext(this); QuadNative.activityOnCreate(this);') | Set-Content -NoNewline '%JAVA_WORK_DIR%\src\%APP_PACKAGE_PATH%\MainActivity.java'"
if errorlevel 1 (
    echo [error] failed to patch MainActivity.java for Android context init
    exit /b 1
)
powershell -NoProfile -Command "(Get-Content -Raw '%JAVA_WORK_DIR%\src\quad_native\QuadNative.java').Replace('public native static void activityOnCreate(Object activity);','public native static void activityOnCreate(Object activity); public native static void initAndroidContext(Object activity);') | Set-Content -NoNewline '%JAVA_WORK_DIR%\src\quad_native\QuadNative.java'"
if errorlevel 1 (
    echo [error] failed to patch QuadNative.java for Android context init
    exit /b 1
)

"%JAVAC_EXE%" --release 8 -classpath "%ANDROID_JAR%" -d "%JAVA_WORK_DIR%\classes" "%JAVA_WORK_DIR%\src\%APP_PACKAGE_PATH%\MainActivity.java" "%JAVA_WORK_DIR%\src\quad_native\QuadNative.java"
if errorlevel 1 (
    echo [error] javac failed
    exit /b 1
)
"%JAR_EXE%" --create --file "%JAVA_WORK_DIR%\classes.jar" -C "%JAVA_WORK_DIR%\classes" .
if errorlevel 1 (
    echo [error] jar packaging failed
    exit /b 1
)
call "%D8_EXE%" --release --lib "%ANDROID_JAR%" --output "%JAVA_WORK_DIR%\dex" "%JAVA_WORK_DIR%\classes.jar"
if errorlevel 1 (
    echo [error] d8 failed
    exit /b 1
)
if not exist "%JAVA_WORK_DIR%\dex\classes.dex" (
    echo [error] classes.dex not generated
    exit /b 1
)

echo [3/7] Building release APK (arm64-v8a) ...
cargo apk build --release --lib --target aarch64-linux-android
if errorlevel 1 (
    echo [error] cargo apk build failed
    exit /b 1
)

echo [4/7] Finding APK artifact ...
set "RAW_APK="
for /f "delims=" %%F in ('dir /b /s /a-d /o-d "target\release\apk\*.apk" 2^>nul') do (
    set "RAW_APK=%%F"
    goto :found_apk
)
for /f "delims=" %%F in ('dir /b /s /a-d /o-d "target\android-artifacts\*.apk" 2^>nul') do (
    set "RAW_APK=%%F"
    goto :found_apk
)
:found_apk
if "%RAW_APK%"=="" (
    echo [error] no APK found under target\release\apk or target\android-artifacts
    exit /b 1
)
echo     raw apk: %RAW_APK%

echo [5/7] Injecting classes.dex into APK ...
if exist "build\android" rmdir /s /q "build\android"
mkdir "build\android" >nul
copy /Y "%JAVA_WORK_DIR%\dex\classes.dex" "build\android\classes.dex" >nul
pushd "build\android"
"%AAPT_EXE%" add "%RAW_APK%" "classes.dex"
if errorlevel 1 (
    popd
    echo [error] aapt add classes.dex failed
    exit /b 1
)
popd

echo [6/7] Aligning APK ...
if exist "build\android" rmdir /s /q "build\android"
mkdir "build\android" >nul
set "ALIGNED_APK=build\android\in_falsus_editor-arm64-v8a-aligned.apk"
set "SIGNED_APK=build\android\in_falsus_editor-arm64-v8a-signed.apk"
"%ZIPALIGN_EXE%" -v -p 4 "%RAW_APK%" "%ALIGNED_APK%"
if errorlevel 1 (
    echo [error] zipalign failed
    exit /b 1
)

echo [7/7] Signing APK ...
call "%APKSIGNER_EXE%" sign ^
--ks "sign\rotaeno.jks" ^
--ks-key-alias "rotaeno" ^
--ks-pass pass:"rotaeno" ^
--ks-type JKS ^
--out "%SIGNED_APK%" ^
"%ALIGNED_APK%"
if errorlevel 1 (
    echo [error] apksigner sign failed
    exit /b 1
)

call "%APKSIGNER_EXE%" verify --print-certs "%SIGNED_APK%"
if errorlevel 1 (
    echo [error] apksigner verify failed
    exit /b 1
)

echo.
echo ========================================
echo   Android APK build complete
echo   Output: %SIGNED_APK%
echo ========================================
exit /b 0
