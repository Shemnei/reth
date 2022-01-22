# Source: https://github.com/getreu/tp-note/blob/master/build-env/Dockerfile

# Unauthorized:
#ARG BASE_IMAGE=debian/bookworm-20211220-slim
ARG BASE_IMAGE=rust:1.58.0-slim-bullseye

FROM $BASE_IMAGE as planner
WORKDIR /home/dockeruser/project

RUN set -eux; \
	apt-get update; \
	apt-get install -y --no-install-recommends \
		git \
		autoconf automake autotools-dev curl python3 libmpc-dev libmpfr-dev \
		libgmp-dev gawk build-essential bison flex texinfo gperf libtool \
		patchutils bc zlib1g-dev libexpat-dev \
		build-essential \
		;

RUN set -eux; \
	git clone https://github.com/riscv-collab/riscv-gnu-toolchain.git /tmp/riscv-gnu-toolchain ; \
	cd /tmp/riscv-gnu-toolchain ; \
	./configure --prefix="/opt/riscv" --enable-multilib ; \
	make ; \
	cd /home/dockeruser/project ; \
	rm -rf /tmp/riscv-gnu-toolchain ;


ENV PATH="${PATH}:/opt/riscv/bin" RISCV="/opt/riscv"

#RUN set -eux; \
#	git clone https://github.com/riscv-software-src/riscv-tests.git riscv-tests ; \
#	cd riscv-tests ; \
#	git submodule update --init --recursive ; \
#	autoconf ; \
#	./configure --prefix="$RISCV/target" ; \
#	make ;
