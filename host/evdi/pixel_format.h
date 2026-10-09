#ifndef BLENT_PIXEL_FORMAT_H
#define BLENT_PIXEL_FORMAT_H
/* Byte order of validated packed four-byte pixels; alpha/X is ignored by NV12.
 * Independent of OS pixel-format constants and capture allocation ownership. */
typedef enum { PIXEL_BGRA, PIXEL_RGBA } pixel_order_t;
static inline int pixel_order_valid(pixel_order_t order) {
    if (order == PIXEL_BGRA) return 1;
    return order == PIXEL_RGBA;
}
#endif
