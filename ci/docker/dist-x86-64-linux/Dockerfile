FROM debian:buster

WORKDIR /checkout

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

# Download and install nvm
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - 
RUN apt-get install -y nodejs

RUN npm i -g pnpm

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="$PATH:/root/.cargo/bin"

ENV EDITION community

ENV CARGO_NET_GIT_FETCH_WITH_CLI true
ENV SCRIPT "cargo dev dist --gui"
RUN mkdir /scripts && touch /scripts/command.sh && chmod +x /scripts/command.sh
RUN echo 'pwd && ls -al' >> /scripts/command.sh
RUN echo 'sh -x -c "$SCRIPT"' >> /scripts/command.sh
