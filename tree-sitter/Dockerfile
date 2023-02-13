FROM node

RUN apt-get update \
    && apt-get install -y clang-11 build-essential vim

USER node

ENV CXX=cc
ENV HOME=/home/node
ENV PATH=$HOME/.npm-global/bin:$PATH

RUN mkdir $HOME/.npm-global \
    && npm config set prefix $HOME/.npm-global \
    && export PATH=$PATH:./node_modules/.bin \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && . "$HOME/.cargo/env" \
    && cargo install tree-sitter-cli
