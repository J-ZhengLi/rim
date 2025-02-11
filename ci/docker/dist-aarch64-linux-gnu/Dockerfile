# 使用 Ubuntu 20.04 版本作为基础镜像
FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive

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
      librsvg2-dev \
      perl \
      gcc-aarch64-linux-gnu

# 安装 Rust（确保是适用于 ARM 的目标）
COPY ci/scripts/install-rust.sh /scripts/
RUN chmod +x /scripts/install-rust.sh && \
    /scripts/install-rust.sh
ENV PATH=$PATH:/root/.cargo/bin

RUN rustup target add aarch64-unknown-linux-gnu
ENV HOST_TRIPPLE aarch64-unknown-linux-gnu
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER aarch64-linux-gnu-gcc

# 安装 Node.js 18.x
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash - && \
    apt-get install -y nodejs
RUN npm set strict-ssl false && npm install -g pnpm

# 安装 Tauri CLI
# RUN cargo install tauri-cli@1.6.4
COPY ci/scripts/install-tauri-cli.sh /scripts/
RUN chmod +x /scripts/install-tauri-cli.sh && \
    /scripts/install-tauri-cli.sh

# 执行项目构建
CMD cargo dev vendor --target aarch64-unknown-linux-gnu --name community && \
    cargo dev dist --cli --target aarch64-unknown-linux-gnu --name community
