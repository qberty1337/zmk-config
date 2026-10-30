#!/usr/bin/env bash
# Runs inside zmkfirmware/zmk-build-arm (started by build-local.ps1).
# Same steps as zmk's build-user-config.yml (CI), for every build.yaml `include` entry.
# Args: optional artifact-name substrings, e.g. `right left_peripheral`. None = build all.
set -euo pipefail
cfg=/zmk-config
cd /work # named docker volume: west workspace + build dirs survive between runs

mkdir -p config
cp "$cfg/config/west.yml" config/west.yml
[ -d .west ] || west init -l config
# ponytail: only re-fetches when west.yml changes, so `revision: main` modules go stale.
# Refresh them with: docker volume rm zmk-west
if ! cmp -s config/west.yml .west/west.yml.done; then
    west update --fetch-opt=--filter=tree:0
    cp config/west.yml .west/west.yml.done
fi
west zephyr-export >/dev/null # registers in ~/.cmake, which dies with the container

# ponytail: `include:` only; add top-level board x shield arrays if build.yaml ever uses them
mapfile -t rows < <(python3 - "$cfg/build.yaml" <<'PY'
import sys, yaml
for e in yaml.safe_load(open(sys.argv[1]))["include"]:
    print("\x1f".join(e.get(k) or "" for k in ("board", "shield", "snippet", "cmake-args", "artifact-name")))
PY
)

mkdir -p "$cfg/build"
built=()
for row in "${rows[@]}"; do
    IFS=$'\x1f' read -r board shield snippet cmake_args artifact <<<"$row"
    artifact=${artifact:-${shield:+$shield-}$board-zmk}
    if [ $# -gt 0 ]; then
        hit=
        for f in "$@"; do [[ $artifact == *"$f"* ]] && hit=1; done
        [ -n "$hit" ] || continue
    fi

    echo "=== $artifact ($board: $shield)"
    eval "extra=($cmake_args)" # build.yaml writes \"..\" for a shell to unescape, as CI does
    west build -p -s zmk/app -d "build/$artifact" -b "$board" ${snippet:+-S "$snippet"} -- \
        -DZMK_CONFIG="$cfg/config" ${shield:+-DSHIELD="$shield"} -DZMK_EXTRA_MODULES="$cfg" \
        "${extra[@]}" </dev/null
    cp "build/$artifact/zephyr/zmk.uf2" "$cfg/build/$artifact.uf2"
    built+=("build/$artifact.uf2")
done

[ ${#built[@]} -gt 0 ] || { echo "no build.yaml artifact matches: $*" >&2; exit 1; }
printf 'built %s\n' "${built[@]}"
