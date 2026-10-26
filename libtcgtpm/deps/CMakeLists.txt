cmake_minimum_required(VERSION 3.16.3)
project(libtpms_bare C)

set(LIBTPMS_SRC ${CMAKE_CURRENT_SOURCE_DIR}/libtpms/src)
set(LIBTPMS_INC ${CMAKE_CURRENT_SOURCE_DIR}/libtpms/include)
set(TPMCMD ${LIBTPMS_SRC}/tpm2/TPMCmd)

# Use libtpms's own TpmConfiguration (it has TDES and other libtpms-specific
# algorithm settings that must match the libtpms source code)
set(TPMCFG_DIR ${TPMCMD}/TpmConfiguration)

# --- Source files ---
# TPM2 core library (from embedded tpm-20-ref)
file(GLOB TPM_COMMAND_SRCS  ${TPMCMD}/tpm/src/command/*/*.c)
file(GLOB TPM_CRYPT_SRCS    ${TPMCMD}/tpm/src/crypt/*.c
                            ${TPMCMD}/tpm/src/crypt/ecc/*.c
                            ${TPMCMD}/tpm/src/crypt/math/*.c)
# Exclude DES support (not available in bare-metal OpenSSL, ALG_TDES disabled)
list(FILTER TPM_CRYPT_SRCS EXCLUDE REGEX "CryptDes\\.c$")
file(GLOB TPM_EVENTS_SRCS   ${TPMCMD}/tpm/src/events/*.c)
file(GLOB TPM_MAIN_SRCS     ${TPMCMD}/tpm/src/main/*.c)
file(GLOB TPM_SUBSYS_SRCS   ${TPMCMD}/tpm/src/subsystem/*.c)
file(GLOB TPM_SUPPORT_SRCS  ${TPMCMD}/tpm/src/support/*.c)
file(GLOB TPM_X509_SRCS     ${TPMCMD}/tpm/src/X509/*.c)

# Crypto libraries
set(TPM_CRYPTO_SRCS
    ${TPMCMD}/tpm/cryptolibs/EccRef/TpmEcc_Signature_ECDSA.c
    ${TPMCMD}/tpm/cryptolibs/RsaRef/PrimeData.c
    ${TPMCMD}/tpm/cryptolibs/TpmBigNum/BnConvert.c
    ${TPMCMD}/tpm/cryptolibs/TpmBigNum/BnEccConstants.c
    ${TPMCMD}/tpm/cryptolibs/TpmBigNum/BnMath.c
    ${TPMCMD}/tpm/cryptolibs/TpmBigNum/BnMemory.c
    ${TPMCMD}/tpm/cryptolibs/TpmBigNum/TpmBigNumThunks.c
    # OpenSSL backend
    ${TPMCMD}/tpm/cryptolibs/Ossl/BnToOsslMath.c
    ${TPMCMD}/tpm/cryptolibs/Ossl/ExpDCache.c
    ${TPMCMD}/tpm/cryptolibs/Ossl/Helpers.c
    ${TPMCMD}/tpm/cryptolibs/Ossl/TpmToOsslDesSupport.c
    ${TPMCMD}/tpm/cryptolibs/Ossl/TpmToOsslSupport.c
)

# Platform library
file(GLOB TPM_PLATFORM_SRCS ${TPMCMD}/Platform/src/*.c)

# TPMCmdp.c provides the _rpc__* functions used by tpm_tpm2_interface.c
set(TPM_SIMULATOR_SRCS
    ${TPMCMD}/Simulator/src/TPMCmdp.c
)

# TpmConfiguration vendor handlers (from libtpms's embedded TpmConfiguration)
set(TPM_VENDOR_SRCS
    ${TPMCMD}/TpmConfiguration/TpmVendorCommandHandlers/Vendor_TCG_Test.c
)

# libtpms command wrappers (replace the glob from Makefile.am)
set(LIBTPMS_TPM2_SRCS
    ${LIBTPMS_SRC}/tpm2/ACTCommands.c
    ${LIBTPMS_SRC}/tpm2/AsymmetricCommands.c
    ${LIBTPMS_SRC}/tpm2/AttestationCommands.c
    ${LIBTPMS_SRC}/tpm2/AuditCommands.c
    ${LIBTPMS_SRC}/tpm2/CapabilityCommands.c
    ${LIBTPMS_SRC}/tpm2/ClockCommands.c
    ${LIBTPMS_SRC}/tpm2/ContextCommands.c
    ${LIBTPMS_SRC}/tpm2/DictionaryCommands.c
    ${LIBTPMS_SRC}/tpm2/DuplicationCommands.c
    ${LIBTPMS_SRC}/tpm2/EACommands.c
    ${LIBTPMS_SRC}/tpm2/EphemeralCommands.c
    ${LIBTPMS_SRC}/tpm2/HashCommands.c
    ${LIBTPMS_SRC}/tpm2/HierarchyCommands.c
    ${LIBTPMS_SRC}/tpm2/IntegrityCommands.c
    ${LIBTPMS_SRC}/tpm2/ManagementCommands.c
    ${LIBTPMS_SRC}/tpm2/NVCommands.c
    ${LIBTPMS_SRC}/tpm2/ObjectCommands.c
    ${LIBTPMS_SRC}/tpm2/RandomCommands.c
    ${LIBTPMS_SRC}/tpm2/SessionCommands.c
    ${LIBTPMS_SRC}/tpm2/SigningCommands.c
    ${LIBTPMS_SRC}/tpm2/StartupCommands.c
    ${LIBTPMS_SRC}/tpm2/SymmetricCommands.c
    ${LIBTPMS_SRC}/tpm2/TestingCommands.c
    ${LIBTPMS_SRC}/tpm2/Unmarshal.c
    ${LIBTPMS_SRC}/tpm2/BackwardsCompatibilityBitArray.c
    ${LIBTPMS_SRC}/tpm2/BackwardsCompatibilityObject.c
    ${LIBTPMS_SRC}/tpm2/LibtpmsCallbacks.c
    ${LIBTPMS_SRC}/tpm2/NVMarshal.c
    ${LIBTPMS_SRC}/tpm2/RuntimeAlgorithm.c
    ${LIBTPMS_SRC}/tpm2/RuntimeAttributes.c
    ${LIBTPMS_SRC}/tpm2/RuntimeCommands.c
    ${LIBTPMS_SRC}/tpm2/RuntimeProfile.c
    ${LIBTPMS_SRC}/tpm2/RuntimeUtils.c
    ${LIBTPMS_SRC}/tpm2/StateMarshal.c
    ${LIBTPMS_SRC}/tpm2/Volatile.c
)

# libtpms library API layer
set(LIBTPMS_API_SRCS
    ${LIBTPMS_SRC}/disabled_interface.c
    ${LIBTPMS_SRC}/tpm_debug.c
    ${LIBTPMS_SRC}/tpm_library.c
    ${LIBTPMS_SRC}/tpm_memory.c
    ${LIBTPMS_SRC}/tpm_nvfile.c
    ${LIBTPMS_SRC}/tpm_tpm2_interface.c
    ${LIBTPMS_SRC}/tpm_tpm2_tis.c
)

add_library(tpms STATIC
    ${TPM_COMMAND_SRCS}
    ${TPM_CRYPT_SRCS}
    ${TPM_EVENTS_SRCS}
    ${TPM_MAIN_SRCS}
    ${TPM_SUBSYS_SRCS}
    ${TPM_SUPPORT_SRCS}
    ${TPM_X509_SRCS}
    ${TPM_CRYPTO_SRCS}
    ${TPM_PLATFORM_SRCS}
    ${TPM_SIMULATOR_SRCS}
    ${TPM_VENDOR_SRCS}
    ${LIBTPMS_TPM2_SRCS}
    ${LIBTPMS_API_SRCS}
)

target_compile_definitions(tpms PRIVATE
    _POSIX_
    TPM_POSIX
    TPM_LIBTPMS_CALLBACKS
    TPM_NV_DISK
    FILE_BACKED_NV=YES
    VTPM=YES
    # Crypto library selection (OpenSSL for all)
    HASH_LIB=Ossl
    SYM_LIB=Ossl
    MATH_LIB=TpmBigNum
    BN_MATH_LIB=Ossl
)

target_include_directories(tpms PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}    # for config.h (libtpms-config.h renamed to config.h via -include)
    ${LIBTPMS_INC}/libtpms
    ${LIBTPMS_SRC}
    ${LIBTPMS_SRC}/tpm2
    ${TPMCMD}/Platform/include
    ${TPMCMD}/Simulator/include
    ${TPMCMD}/Simulator/include/prototypes
    ${TPMCFG_DIR}
    ${TPMCMD}/tpm/include
    ${TPMCMD}/tpm/include/private
    ${TPMCMD}/tpm/include/private/prototypes
    ${TPMCMD}/tpm/include/tpm_public
    ${TPMCMD}/tpm/include/platform_interface
    ${TPMCMD}/tpm/include/platform_interface/prototypes
    ${TPMCMD}/tpm/cryptolibs/common/include
    ${TPMCMD}/tpm/cryptolibs/TpmBigNum/include
    ${TPMCMD}/tpm/cryptolibs/TpmBigNum/include/TpmBigNum
    ${TPMCMD}/tpm/cryptolibs
    ${TPMCMD}/tpm/cryptolibs/Ossl
    ${TPMCMD}/tpm/cryptolibs/Ossl/include
    ${TPMCMD}/tpm/cryptolibs/Ossl/include/Ossl
)

# config.h is found via include path (libtpms sources do #include <config.h>)
# tpm_library_conf.h is found because we add LIBTPMS_SRC to include dirs
target_include_directories(tpms PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/libtpms-bare-config
)
