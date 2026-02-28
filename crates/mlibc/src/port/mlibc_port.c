// Minimal std-port style shim compiled with cc.
// Real mlibc sources are fetched by build.rs; this file acts as a tiny port surface.

#include <stddef.h>
#include <stdint.h>

extern long __mlibc_rs_write(int fd, const void *buf, size_t len);
extern long __mlibc_rs_read(int fd, void *buf, size_t len);
extern void *__mlibc_rs_memmap(size_t len);

long mlibc_sys_write(int fd, const void *buf, size_t len) {
    return __mlibc_rs_write(fd, buf, len);
}

long mlibc_sys_read(int fd, void *buf, size_t len) {
    return __mlibc_rs_read(fd, buf, len);
}

void *mlibc_sys_memmap(size_t len) {
    return __mlibc_rs_memmap(len);
}

long mlibc_stub_unimplemented(void) {
    return -38;
}
