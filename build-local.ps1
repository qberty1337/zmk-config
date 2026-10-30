# Build firmware locally in Docker, same as the GitHub Action. Output: build\<artifact-name>.uf2
# Usage: .\build-local.ps1                         # everything in build.yaml
#        .\build-local.ps1 right left_peripheral   # only artifact names containing these
# First run downloads zephyr/zmk/modules into the "zmk-west" docker volume (a few minutes).
docker run --rm -v zmk-west:/work -v "${PSScriptRoot}:/zmk-config" `
    zmkfirmware/zmk-build-arm:stable bash /zmk-config/build-local.sh @args
exit $LASTEXITCODE
