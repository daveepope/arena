FROM gcr.io/oss-fuzz-base/base-builder-rust@sha256:5fc0de2bcb2eee5ab04db05ef175f215ab0fc30deb0c509403125cefd2fc8689

COPY . $SRC/arena
WORKDIR $SRC/arena
COPY .clusterfuzzlite/build.sh $SRC/
