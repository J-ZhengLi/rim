# 使用 Ubuntu 20.04 版本作为基础镜像
FROM messense/rust-musl-cross:x86_64-musl

ENV DEBIAN_FRONTEND=noninteractive

RUN ld --version

# 更新系统并安装必要的软件包
RUN apt-get update && \
    apt-get install -y \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.0-dev \
    libappindicator3-dev \
    librsvg2-dev

# 安装 Node.js 18.x
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash - && \
    apt-get install -y nodejs
RUN npm set strict-ssl false && npm install -g pnpm

# 安装 Tauri CLI
COPY ci/scripts/install-tauri-cli.sh /scripts/
RUN chmod +x /scripts/install-tauri-cli.sh && \
    /scripts/install-tauri-cli.sh

# 执行项目构建
CMD cargo dev vendor --for x86_64-unknown-linux-gnu && \
    cargo dev dist --cli --for x86_64-unknown-linux-gnu --target x86_64-unknown-linux-musl && \
    cargo dev dist --gui --for x86_64-unknown-linux-gnu --target x86_64-unknown-linux-gnu
