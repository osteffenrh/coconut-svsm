CC ?= gcc

ifdef RELEASE
LIBTPMS_CFLAGS = -O3 -DDEBUG=NO
else
LIBTPMS_CFLAGS = -g -O0 -DDEBUG=YES
endif

# Allow user to not link in libcrt
USE_LIBCRT ?= 1

# OpenSSL and libcrt are built externally (by cocoon-tpm-ossl-bare-sys
# and the libcrt crate respectively).
# The following must be provided by the caller:
#   OPENSSL_INCLUDE_DIR, OPENSSL_SRC_INCLUDE_DIR, OPENSSL_LIB_DIR
#   LIBCRT_INCLUDE_DIR

ROOT_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
DEPS_DIR := $(ROOT_DIR)/deps
PATCHES_DIR := $(ROOT_DIR)/patches
LIBTPMS_DIR := $(DEPS_DIR)/libtpms
OUT_DIR ?= $(ROOT_DIR)
# Ensure OUT_DIR is an absolute path because we use it to build include paths
# used by different projects
override OUT_DIR := $(abspath $(OUT_DIR))

# Include CARGO_MAKEFLAGS for jobserver coordination if available
# Otherwise use -j with NUM_JOBS
ifneq ($(CARGO_MAKEFLAGS),)
MAKEFLAGS += $(CARGO_MAKEFLAGS)
else
# Use NUM_JOBS from Cargo build script environment, fallback to nproc
NUM_JOBS ?= $(shell nproc)
MAKEFLAGS += -j$(NUM_JOBS)
endif

LIBTPMS_BUILD_DIR = $(OUT_DIR)/libtpms-build

LIBCRYPTO = $(OPENSSL_LIB_DIR)/libcrypto.a

LIBTPMS = $(LIBTPMS_BUILD_DIR)/libtpms.a

LIBTPMS_MAKEFILE = $(LIBTPMS_BUILD_DIR)/Makefile

ifeq ($(USE_LIBCRT),1)
LIBTPMS_CFLAGS += -static -nostdinc -fno-stack-protector -mno-sse -mno-red-zone
LIBTPMS_CFLAGS += -I$(LIBCRT_INCLUDE_DIR)
endif

LIBTPMS_CFLAGS += -fPIE
LIBTPMS_CFLAGS += -I$(OPENSSL_INCLUDE_DIR)
LIBTPMS_CFLAGS += -I$(OPENSSL_SRC_INCLUDE_DIR)

.NOTPARALLEL:

PATCHES_APPLIED = $(LIBTPMS_BUILD_DIR)/.patches-applied

all: $(LIBTPMS)

$(PATCHES_APPLIED): $(wildcard $(PATCHES_DIR)/*.patch)
	mkdir -p $(LIBTPMS_BUILD_DIR)
	cd $(LIBTPMS_DIR) && git checkout -- . 2>/dev/null || true
	cd $(LIBTPMS_DIR) && for p in $(PATCHES_DIR)/*.patch; do \
		git apply "$$p" || exit 1; \
	done
	sed -e 's/@LIBTPMS_VER_MAJOR@/0/' \
	    -e 's/@LIBTPMS_VER_MINOR@/11/' \
	    -e 's/@LIBTPMS_VER_MICRO@/0/' \
	    $(LIBTPMS_DIR)/include/libtpms/tpm_library.h.in \
	    > $(LIBTPMS_DIR)/include/libtpms/tpm_library.h
	touch $@

$(LIBTPMS): $(LIBTPMS_MAKEFILE) $(LIBCRYPTO)
	$(MAKE) -C $(LIBTPMS_BUILD_DIR) tpms

$(LIBTPMS_MAKEFILE): $(LIBCRYPTO) $(PATCHES_APPLIED)
	mkdir -p $(LIBTPMS_BUILD_DIR)
	(cd $(LIBTPMS_BUILD_DIR) && \
		cmake \
			-DCMAKE_SYSTEM_NAME=Generic	\
			-DCMAKE_C_COMPILER="$(CC)"	\
			-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY \
			-DCMAKE_C_FLAGS="${LIBTPMS_CFLAGS}" \
			-DOPENSSL_CRYPTO_LIBRARY="$(LIBCRYPTO)" \
			-DOPENSSL_INCLUDE_DIR="$(OPENSSL_INCLUDE_DIR)" \
			-Duser_TpmConfiguration_Dir="$(DEPS_DIR)/TpmConfiguration" \
			$(DEPS_DIR))

clean:
	[ ! -f $(LIBTPMS_MAKEFILE) ] || $(MAKE) -C $(LIBTPMS_BUILD_DIR) clean

distclean: clean
	rm -rf $(LIBTPMS_BUILD_DIR)
	cd $(LIBTPMS_DIR) && git checkout -- . 2>/dev/null || true

.PHONY: all clean distclean configure_libtpms
configure_libtpms: $(LIBTPMS_MAKEFILE)
