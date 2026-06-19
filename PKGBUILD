pkgname=roughnote
pkgver=0.1.0
pkgrel=1
pkgdesc="A native Linux drawing and comprehensive note-taking application built with Rust and GTK4"
arch=('x86_64')
url="https://github.com/kamallearner123/ClassBoard"
license=('MIT')
depends=('gtk4')
makedepends=('cargo' 'git')
source=("${pkgname}::git+${url}.git")
sha256sums=('SKIP')

build() {
    cd "${pkgname}"
    cargo build --release --locked
}

package() {
    cd "${pkgname}"
    
    # Binary
    install -Dm755 target/release/roughnote "${pkgdir}/usr/bin/roughnote"
    
    # Desktop Entry
    install -d "${pkgdir}/usr/share/applications"
    cat <<EOF > "${pkgdir}/usr/share/applications/roughnote.desktop"
[Desktop Entry]
Name=RoughNote
Comment=Native vector drawing and note-taking application
Exec=roughnote
Icon=roughnote
Terminal=false
Type=Application
Categories=Utility;Graphics;Office;
EOF

    # Icon
    if [ -f assets/logo.png ]; then
        install -Dm644 assets/logo.png "${pkgdir}/usr/share/pixmaps/roughnote.png"
    fi
}