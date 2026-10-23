/* SPDX-License-Identifier: MIT */

#include <stdlib.h>
#include <string.h>
#include <stddef.h>

char *strndup(const char *s, size_t n)
{
	size_t l = strlen(s);
	if (l > n) l = n;
	char *d = malloc(l+1);
	if (!d) return NULL;
	memcpy(d, s, l);
	d[l] = '\0';
	return d;
}
