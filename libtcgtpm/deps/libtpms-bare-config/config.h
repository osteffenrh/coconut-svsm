/* config.h for bare-metal libtpms build in Coconut-SVSM */
#ifndef LIBTPMS_CONFIG_H
#define LIBTPMS_CONFIG_H

#define WITH_TPM2 1
/* No TPM 1.2 support */
/* #undef WITH_TPM1 */

#define LIBTPMS_USE_OPENSSL 1

#define PACKAGE "libtpms"
#define VERSION "0.11.0"

/* libtpms version */
#define LIBTPMS_VER_MAJOR 0
#define LIBTPMS_VER_MINOR 11
#define LIBTPMS_VER_MICRO 0

/* Pull in the library configuration constants (normally force-included
   via -include by autotools) */
#include "tpm_library_conf.h"

#endif /* LIBTPMS_CONFIG_H */
