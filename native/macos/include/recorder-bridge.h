#ifndef RECORDER_BRIDGE_H
#define RECORDER_BRIDGE_H

#include <stdint.h>

/*
 * C ABI of the macOS capture bridge. A handle belongs to one control thread;
 * every function except the block callback runs on that thread.
 */
typedef struct recorder_handle recorder_handle;

enum recorder_result {
    RECORDER_OK = 0,
    RECORDER_INVALID_ARGUMENT = 1,
    RECORDER_SOURCE_UNAVAILABLE = 2,
    RECORDER_INVALID_STATE = 3,
    /* Microphone access is not granted. macOS delivers silence without it. */
    RECORDER_PERMISSION_DENIED = 4,
    /* A Core Audio call failed; recorder_last_os_status returns its OSStatus. */
    RECORDER_NATIVE_ERROR = 5
};

enum recorder_source_kind {
    /* The current default input device. */
    RECORDER_SOURCE_MICROPHONE = 1,
    /* Everything the Mac plays, through a global process tap clocked by the default
     * output device. Without the System Audio Recording grant it delivers zeros. */
    RECORDER_SOURCE_SYSTEM = 2
};

enum recorder_sample_format {
    /* Packed, interleaved, native-endian 32-bit float. */
    RECORDER_SAMPLE_F32 = 1
};

enum recorder_access {
    RECORDER_ACCESS_NOT_DETERMINED = 0,
    RECORDER_ACCESS_DENIED = 1,
    RECORDER_ACCESS_GRANTED = 2
};

enum recorder_block_flags {
    RECORDER_BLOCK_HOST_TIME_VALID = 1u << 0,
    RECORDER_BLOCK_SAMPLE_TIME_VALID = 1u << 1,
    /* Frames were lost between the previous block and this one. */
    RECORDER_BLOCK_DISCONTINUITY = 1u << 2
};

/* Format negotiated by recorder_prepare. Blocks never exceed max_block_frames. */
typedef struct recorder_format {
    uint32_t sample_rate_hz;
    uint32_t channel_count;
    uint32_t sample_format;
    uint32_t max_block_frames;
} recorder_format;

/*
 * One block of captured frames. `samples` holds frame_count * channel_count
 * values and is borrowed: it is valid only during the callback.
 * host_time_ns is on the mach_absolute_time clock converted to nanoseconds.
 */
typedef struct recorder_block {
    const float *samples;
    uint64_t host_time_ns;
    double sample_time;
    uint32_t frame_count;
    uint32_t flags;
} recorder_block;

_Static_assert(sizeof(recorder_format) == 16, "recorder_format layout");
_Static_assert(sizeof(recorder_block) == 32, "recorder_block layout");

/*
 * Runs on the real-time audio thread: copy the samples and return without
 * blocking, allocating, locking, or doing I/O.
 */
typedef void (*recorder_block_callback)(void *context, const recorder_block *block);

/* Reports microphone access; when `request` is nonzero and access is not yet
 * determined, asks the user and blocks until they answer. */
int32_t recorder_microphone_access(uint32_t request, uint32_t *access);
/* Counts the available microphones: 1 when a default input device exists. */
int32_t recorder_list(uint32_t *count);
int32_t recorder_create(recorder_handle **handle);
/* Opens the source and writes the negotiated format. The microphone requires granted
 * access. A handle captures one source; it switches source only when idle. */
int32_t recorder_prepare(recorder_handle *handle, uint32_t source_kind, recorder_format *format);
/* Starts delivering blocks to `callback` with `context`. */
int32_t recorder_start(recorder_handle *handle, recorder_block_callback callback, void *context);
/* Idempotent. When it returns, no callback is running or will run again, and
 * the handle is back in the created state. For the system source, the IOProc,
 * aggregate device, and tap are destroyed in that order. */
int32_t recorder_stop(recorder_handle *handle);
/* Stops the handle if needed, then frees it. */
int32_t recorder_destroy(recorder_handle *handle);
/* The OSStatus of the last failed Core Audio call on the control thread, or 0. */
int32_t recorder_last_os_status(recorder_handle *handle, int32_t *status);

#endif
