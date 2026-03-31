# Maintainer: Your Name <you@example.com>
pkgname=canvaswm
pkgver=0.1.0
pkgrel=1
pkgdesc="An infinite-canvas Wayland compositor"
arch=('x86_64' 'aarch64')
url="https://github.com/hades/canvaswm"
license=('MIT')
depends=(
    'libgl'
    'libxkbcommon'
    'wayland'
    'libinput'
    'libseat'
    'mesa'
    'xwayland'
    'netcat'  # for canvaswm-msg
)
makedepends=('cargo' 'pkg-config')
source=("$pkgname::git+$url.git")
sha256sums=('SKIP')

prepare() {
    cd "$pkgname"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$pkgname"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release --all-features
}

check() {
    cd "$pkgname"
    export RUSTUP_TOOLCHAIN=stable
    cargo test --frozen
}

package() {
    cd "$pkgname"
    install -Dm755 target/release/canvaswm  "$pkgdir/usr/bin/canvaswm"
    install -Dm755 extras/canvaswm-msg       "$pkgdir/usr/bin/canvaswm-msg"
    install -Dm644 LICENSE                   "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    if [[ -f "example/canvaswm.toml" ]]; then
        install -Dm644 example/canvaswm.toml "$pkgdir/usr/share/canvaswm/canvaswm.toml"
    fi
}
