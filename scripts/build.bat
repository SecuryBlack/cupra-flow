@echo off
echo ======================================
echo  CupraFlow - Build Script
echo ======================================

REM Verificar Rust
cargo --version > nul 2>&1
if errorlevel 1 (
    echo ERROR: Rust/Cargo no encontrado. Instala Rust desde https://rustup.rs/
    exit /b 1
)

echo.
echo [1/3] Compilando en modo release...
cargo build --release

if errorlevel 1 (
    echo ERROR: Fallo en la compilacion
    exit /b 1
)

echo.
echo [2/3] Verificando binario...
if exist "target\release\cupraflow.exe" (
    echo Binario generado: target\release\cupraflow.exe
    target\release\cupraflow.exe --version
) else (
    echo ERROR: No se encontro el binario
    exit /b 1
)

echo.
echo [3/3] Copiando assets...
if not exist "dist" mkdir dist
copy "target\release\cupraflow.exe" "dist\"
xcopy /E /I /Y "config" "dist\config\" > nul 2>&1
xcopy /E /I /Y "web" "dist\web\" > nul 2>&1

echo.
echo ======================================
echo  Build completado exitosamente
echo  Ejecutable: dist\cupraflow.exe
echo ======================================

pause