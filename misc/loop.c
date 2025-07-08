#include <stdint.h>
#include <stdlib.h>

#define BUFSIZE 1024 * 1024

int main() {
	volatile uint8_t *dest = malloc(BUFSIZE);

	for (uint64_t i = 0; i < BUFSIZE; i++) {
		dest[i] = (uint8_t) i;
	}
}
