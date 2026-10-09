/* Test boundary declarations only; no dependency implementation is audited. */
#include <stddef.h>
typedef struct ZSTD_DCtx_s ZSTD_DCtx;
ZSTD_DCtx *ZSTD_createDCtx(void);
size_t ZSTD_freeDCtx(ZSTD_DCtx *);
size_t ZSTD_decompressDCtx(ZSTD_DCtx *, void *, size_t, const void *, size_t);
