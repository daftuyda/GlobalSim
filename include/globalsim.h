#ifndef GLOBALSIM_H
#define GLOBALSIM_H
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct GlobalSimHandle GlobalSimHandle;

/* Create, use and free each handle on one thread, without concurrent access.
 * Independent threads may each create their own handles.
 * Return values: 0 = success, -1 = error, -2 = wrong thread.
 * Buffer views expire on the next prepare, run or free for their handle. */
GlobalSimHandle *globalsim_new(void);
int32_t globalsim_prepare(GlobalSimHandle *handle, const uint8_t *json, size_t length);
int32_t globalsim_run(GlobalSimHandle *handle, int64_t seed);
const uint8_t *globalsim_data(const GlobalSimHandle *handle);
size_t globalsim_size(const GlobalSimHandle *handle);
const char *globalsim_error(const GlobalSimHandle *handle);
int32_t globalsim_free(GlobalSimHandle *handle);

#ifdef __cplusplus
}
#endif
#endif
