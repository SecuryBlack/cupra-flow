@echo off
setlocal enabledelayedexpansion

echo ===========================================
echo  Cupra-Flow - Test de Servicio Windows
echo ===========================================
echo.

set EXE=%~dp0..\target\release\cupraflow.exe
set SERVICE_NAME=Cupra-Flow
set LOG_DIR=C:\ProgramData\Cupra-Flow

echo [1/6] Instalando servicio...
"%EXE%" install
if %errorlevel% neq 0 (
    echo [ERROR] Fallo la instalacion. Ejecutaste como Administrador?
    goto end
)
echo.

echo [2/6] Verificando servicio en SC...
sc query %SERVICE_NAME%
echo.

echo [3/6] Iniciando servicio...
"%EXE%" start
if %errorlevel% neq 0 (
    echo [WARNING] No se pudo iniciar el servicio.
)
echo.

timeout /t 2 /nobreak >nul

echo [4/6] Consultando estado via SC...
sc query %SERVICE_NAME%
echo.

echo [5/6] Verificando log de servicio...
set "LOG_FILE="
for /f "delims=" %%a in ('dir /b /o-d "%LOG_DIR%\cupraflow.log.*" 2^>nul') do (
    set "LOG_FILE=%%a"
    goto :found_log
)
:found_log
if defined LOG_FILE (
    echo [OK] Log encontrado: %LOG_FILE%
    type "%LOG_DIR%\%LOG_FILE%"
) else (
    if exist "%LOG_DIR%\cupraflow.log" (
        echo [OK] Log encontrado: cupraflow.log
        type "%LOG_DIR%\cupraflow.log"
    ) else (
        echo [INFO] No se encontro log. Esto es normal si el servicio no llego a iniciar.
    )
)
echo.

echo [6/6] Deteniendo y desinstalando servicio...
"%EXE%" stop
"%EXE%" uninstall
echo.

echo ===========================================
echo  Test completado
echo ===========================================

:end
pause
