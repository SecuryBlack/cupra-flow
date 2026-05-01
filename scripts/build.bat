@echo off
cd /d "%~dp0.."
echo ======================================
echo  Cupra-Flow - Build Script
echo ======================================

REM Verificar Rust
cargo --version > nul 2>&1
if errorlevel 1 (
    echo ERROR: Rust/Cargo no encontrado. Instala Rust desde https://rustup.rs/
    exit /b 1
)

echo.
echo [1/4] Compilando en modo release...
cargo build --release

if errorlevel 1 (
    echo ERROR: Fallo en la compilacion
    exit /b 1
)

echo.
echo [2/4] Verificando binario...
if exist "target\release\cupraflow.exe" (
    echo Binario generado: target\release\cupraflow.exe
    target\release\cupraflow.exe --version
) else (
    echo ERROR: No se encontro el binario
    exit /b 1
)

echo.
echo [3/4] Copiando assets a dist\...
if not exist "dist" mkdir dist
copy "target\release\cupraflow.exe" "dist\" > nul
xcopy /E /I /Y "config" "dist\config\" > nul 2>&1
xcopy /E /I /Y "web" "dist\web\" > nul 2>&1

echo.
echo [4/4] Generando paquete ZIP...
set "PKG=cupraflow-x86_64-pc-windows-msvc"
if exist "%PKG%" rmdir /S /Q "%PKG%" > nul 2>&1
mkdir "%PKG%"
copy "target\release\cupraflow.exe" "%PKG%\" > nul
copy "config\config.toml" "%PKG%\" > nul
powershell -Command "Compress-Archive -Path '%PKG%\*' -DestinationPath '%PKG%.zip' -Force"
certutil -hashfile "%PKG%.zip" SHA256 > "%PKG%.zip.sha256.tmp"
findstr /V "CertUtil" "%PKG%.zip.sha256.tmp" > "%PKG%.zip.sha256"
del "%PKG%.zip.sha256.tmp"
if exist "%PKG%" rmdir /S /Q "%PKG%" > nul 2>&1

echo.
echo ======================================
echo  Build completado exitosamente
echo  Ejecutable: dist\cupraflow.exe
echo  Paquete:    %PKG%.zip
echo ======================================

pause
