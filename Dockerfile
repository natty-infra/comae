FROM rust:1.67-buster as build

RUN update-ca-certificates

ENV USER=comae
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

WORKDIR /comae

COPY ./ .

RUN cargo build --release

FROM debian:buster-slim

RUN apt-get install -y ca-certificates

COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group

WORKDIR /comae

RUN chown -R 10001:10001 .

COPY --from=build /comae/target/release/comae ./
RUN mkdir -p /comae/keys

USER comae:comae

CMD ["/comae/comae"]