# Local-only build environment. Application packages contain no container runtime.
FROM docker.io/library/rust@sha256:5c5066e3f3bdd22a5cec7ba22ef0ee6e0bf6eaf63b65b63c9bf25f6f69a5e26a
LABEL org.opencontainers.image.title="Inodune Debian 12 builder" \
      org.opencontainers.image.description="Non-root local build and smoke environment; not a published application image" \
      org.opencontainers.image.licenses="MIT"
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libssl-dev \
    libxdo-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev \
    python3 ca-certificates curl git bubblewrap xvfb webkit2gtk-driver \
    fonts-dejavu-core dbus-x11 xauth binutils dpkg-dev \
    && apt-get clean
RUN useradd --uid 1000 --create-home builder && install -d -o 1000 -g 1000 /workspace /cache/cargo /out
ENV CARGO_HOME=/cache/cargo RUSTUP_HOME=/usr/local/rustup
WORKDIR /workspace
USER 1000:1000
STOPSIGNAL SIGTERM
ENTRYPOINT ["/usr/local/cargo/bin/cargo"]
