/* SPDX-License-Identifier: MIT */

/* Minimal regex.h stub for bare-metal builds.
   The regex functions always fail, which is acceptable because
   runtime profile selection is not used in the SVSM environment. */

#ifndef _REGEX_H
#define _REGEX_H

#include <stddef.h>

#define REG_EXTENDED 1
#define REG_NOSUB    2
#define REG_NOMATCH  1

typedef struct {
    int dummy;
} regex_t;

typedef struct {
    int rm_so;
    int rm_eo;
} regmatch_t;

static inline int regcomp(regex_t *preg, const char *regex, int cflags) {
    (void)preg; (void)regex; (void)cflags;
    return -1;
}

static inline int regexec(const regex_t *preg, const char *string,
                          size_t nmatch, regmatch_t pmatch[], int eflags) {
    (void)preg; (void)string; (void)nmatch; (void)pmatch; (void)eflags;
    return REG_NOMATCH;
}

static inline void regfree(regex_t *preg) {
    (void)preg;
}

#endif /* _REGEX_H */
