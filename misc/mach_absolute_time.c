#include "stdint.h"
#include "stdio.h"
#include "mach/mach_time.h"


int main() {
	uint64_t time = mach_absolute_time();

	printf("Time is %llu\n", time);
	return 0;
}
