/* SPDX-License-Identifier: MIT */

/* Stub netinet/in.h for bare-metal builds.
   Only byte-order macros are needed; on x86_64 these are identity
   or bswap — provide minimal definitions. */

#ifndef _NETINET_IN_H
#define _NETINET_IN_H

#include <stdint.h>

static inline uint16_t htons(uint16_t x) {
    return __builtin_bswap16(x);
}

static inline uint16_t ntohs(uint16_t x) {
    return __builtin_bswap16(x);
}

static inline uint32_t htonl(uint32_t x) {
    return __builtin_bswap32(x);
}

static inline uint32_t ntohl(uint32_t x) {
    return __builtin_bswap32(x);
}

#endif /* _NETINET_IN_H */
