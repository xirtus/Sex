DYNAMIC_INIT

export LDFLAGS="$LDFLAGS -lSDL2 -lorbital -lOSMesa -lstdc++"
ARCH="${TARGET%%-*}"
COOKBOOK_CONFIGURE_FLAGS=(
    --enable-cross-compile
    --target-os=redox
    --arch="${ARCH}"
    --cross_prefix="${TARGET}-"
    --prefix=/usr
    --disable-doc
    --enable-shared
    --disable-static
    --disable-network
    --enable-sdl2
    --enable-zlib
    --enable-encoder=png
    --enable-decoder=png
)
cookbook_configure
mkdir -pv "${COOKBOOK_STAGE}/ui/apps"
cp -v "${COOKBOOK_RECIPE}/manifest" "${COOKBOOK_STAGE}/ui/apps/ffplay"
