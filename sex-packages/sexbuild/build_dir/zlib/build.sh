
function DYNAMIC_INIT {
    echo "sexbuild: DYNAMIC_INIT"
}
function cookbook_configure {
    echo "sexbuild: cookbook_configure"
    if [ -f "./configure" ]; then
        ./configure "${COOKBOOK_CONFIGURE_FLAGS[@]}"
    fi
}
export TARGET="x86_64-sex"
export CC="sexos-cc"
export CXX="sexos-cc"
export AR="llvm-ar"
export NM="llvm-nm"
export STRIP="llvm-strip"
export COOKBOOK_CONFIGURE="./configure"
export COOKBOOK_MAKE="make"
export COOKBOOK_STAGE="stage"
DYNAMIC_INIT
COOKBOOK_CONFIGURE_FLAGS=(--prefix="/usr")
if [ "${COOKBOOK_DYNAMIC}" == "1" ]
then
    COOKBOOK_CONFIGURE_FLAGS+=(--shared)
else
    COOKBOOK_CONFIGURE_FLAGS+=(--static)
fi
# See https://stackoverflow.com/questions/21396988/zlib-build-not-configuring-properly-with-cross-compiler-ignores-ar.
env CHOST="${TARGET}" "${COOKBOOK_CONFIGURE}" "${COOKBOOK_CONFIGURE_FLAGS[@]}"
"${COOKBOOK_MAKE}" -j "$(nproc)"
"${COOKBOOK_MAKE}" install DESTDIR="${COOKBOOK_STAGE}"
solib="${COOKBOOK_STAGE}/usr/lib/libz.so.1.3"
if [ -e "${solib}" ]
then
    patchelf --set-soname 'libz.so.1.3' "${solib}"
fi
