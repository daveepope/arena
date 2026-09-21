FROM gcr.io/oss-fuzz-base/base-builder-rust@sha256:30133d6ddfd9d2649c2821e3ee928e399f17b17dd04697f553bcda403e4f58b2

COPY . $SRC/arena
WORKDIR $SRC/arena
COPY .clusterfuzzlite/build.sh $SRC/
