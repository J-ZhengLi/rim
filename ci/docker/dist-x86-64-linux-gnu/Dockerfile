FROM debian:buster

WORKDIR /build

RUN sed -i 's/deb.debian.org/repo.huaweicloud.com/g' /etc/apt/sources.list

RUN apt-get update -y && \
    apt-get install -y apt-transport-https ca-certificates && \
    apt-get install -y libwebkit2gtk-4.0-dev \
        build-essential \
        curl \
        wget \
        file \
        libssl-dev \
        libgtk-3-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        git

ENV NVM_DIR "$HOME/.nvm"
RUN wget -qO- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
RUN [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

RUN NVM_NODEJS_ORG_MIRROR=https://repo.huaweicloud.com/nodejs/ nvm install --lts && \
    NVM_IOJS_ORG_MIRROR=https://repo.huaweicloud.com/iojs/ nvm use --lts

RUN npm i -g pnpm

ENV RUSTUP_DIST_SERVER "https://rsproxy.cn"
ENV RUSTUP_UPDATE_ROOT "https://rsproxy.cn/rustup"
RUN curl --proto '=https' --tlsv1.2 -sSf https://rsproxy.cn/rustup-init.sh | sh -s -- -y && \
    . "$HOME/.cargo/env"

ENV CARGO_NET_GIT_FETCH_WITH_CLI true
RUN cargo dev vendor --for x86_64-unknown-linux-gnu
RUN cargo dev dist --target x86_64-unknown-linux-gnu --for x86_64-unknown-linux-gnu
