/* SPDX-License-Identifier: MIT */

#include <stdlib.h>
#include <ctype.h>
#include <limits.h>

unsigned long strtoul(const char *nptr, char **endptr, int base)
{
	const char *s = nptr;
	unsigned long acc;
	int c;
	unsigned long cutoff;
	int neg = 0, any, cutlim;

	do {
		c = (unsigned char)*s++;
	} while (isspace(c));

	if (c == '-') {
		neg = 1;
		c = (unsigned char)*s++;
	} else if (c == '+') {
		c = (unsigned char)*s++;
	}

	if ((base == 0 || base == 16) &&
	    c == '0' && (*s == 'x' || *s == 'X')) {
		c = (unsigned char)s[1];
		s += 2;
		base = 16;
	}
	if (base == 0)
		base = c == '0' ? 8 : 10;

	cutoff = ULONG_MAX / (unsigned long)base;
	cutlim = ULONG_MAX % (unsigned long)base;
	for (acc = 0, any = 0;; c = (unsigned char)*s++) {
		if (isdigit(c))
			c -= '0';
		else if (isupper(c))
			c -= 'A' - 10;
		else if (islower(c))
			c -= 'a' - 10;
		else
			break;
		if (c >= base)
			break;
		if (any < 0 || acc > cutoff || (acc == cutoff && (unsigned long)c > (unsigned long)cutlim)) {
			any = -1;
		} else {
			any = 1;
			acc *= (unsigned long)base;
			acc += (unsigned long)c;
		}
	}
	if (any < 0) {
		acc = ULONG_MAX;
	} else if (neg) {
		acc = -acc;
	}
	if (endptr != NULL)
		*endptr = (char *)(any ? s - 1 : nptr);
	return acc;
}

long strtol(const char *nptr, char **endptr, int base)
{
	const char *s = nptr;
	unsigned long acc;
	int c;
	unsigned long cutoff;
	int neg = 0, any, cutlim;

	do {
		c = (unsigned char)*s++;
	} while (isspace(c));

	if (c == '-') {
		neg = 1;
		c = (unsigned char)*s++;
	} else if (c == '+') {
		c = (unsigned char)*s++;
	}

	if ((base == 0 || base == 16) &&
	    c == '0' && (*s == 'x' || *s == 'X')) {
		c = (unsigned char)s[1];
		s += 2;
		base = 16;
	}
	if (base == 0)
		base = c == '0' ? 8 : 10;

	cutoff = neg ? -(unsigned long)LONG_MIN : (unsigned long)LONG_MAX;
	cutlim = cutoff % (unsigned long)base;
	cutoff /= (unsigned long)base;
	for (acc = 0, any = 0;; c = (unsigned char)*s++) {
		if (isdigit(c))
			c -= '0';
		else if (isupper(c))
			c -= 'A' - 10;
		else if (islower(c))
			c -= 'a' - 10;
		else
			break;
		if (c >= base)
			break;
		if (any < 0 || acc > cutoff || (acc == cutoff && (unsigned long)c > (unsigned long)cutlim)) {
			any = -1;
		} else {
			any = 1;
			acc *= (unsigned long)base;
			acc += (unsigned long)c;
		}
	}
	if (any < 0) {
		acc = neg ? (unsigned long)LONG_MIN : (unsigned long)LONG_MAX;
	} else if (neg) {
		acc = -acc;
	}
	if (endptr != NULL)
		*endptr = (char *)(any ? s - 1 : nptr);
	return (long)acc;
}
