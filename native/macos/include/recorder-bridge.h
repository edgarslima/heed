#ifndef RECORDER_BRIDGE_H
#define RECORDER_BRIDGE_H

#include <stdint.h>

/* A handle belongs to one control thread. Stop it before destroying it. */
typedef struct recorder_handle recorder_handle;

enum recorder_result {
    RECORDER_OK = 0,
    RECORDER_INVALID_ARGUMENT = 1,
    RECORDER_SOURCE_UNAVAILABLE = 2,
    RECORDER_INVALID_STATE = 3
};

/* Discovery and capture are unavailable until their native adapters are implemented. */
int32_t recorder_list(uint32_t *count);
int32_t recorder_create(recorder_handle **handle);
int32_t recorder_prepare(recorder_handle *handle);
int32_t recorder_start(recorder_handle *handle);
int32_t recorder_stop(recorder_handle *handle);
int32_t recorder_destroy(recorder_handle *handle);

#endif
