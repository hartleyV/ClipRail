@echo off
cd /d "%~dp0"
cargo build --release
if errorlevel 1 exit /b 1
copy /Y target\release\cliprail.exe ClipRail.exe
echo Built: ClipRail.exe
pause
