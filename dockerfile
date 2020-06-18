FROM ubuntu:18.04 as build

RUN apt update -y && apt install -y curl && curl https://getsubstrate.io -sSf | bash -s -- --fast

RUN source /root/.cargo/env  
RUN rustup update nightly && rustup update stable && rustup target add wasm32-unknown-unknown --toolchain nightly

COPY . /opt

WORKDIR /opt

CMD [ "/bin/bash" ]