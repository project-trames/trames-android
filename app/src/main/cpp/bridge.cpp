// SPDX-License-Identifier: GPL-3.0-or-later
#include <jni.h>
#include <cstdint>
#include <cstddef>
#include <cstdio>

struct Probe { uint32_t schema; uint32_t flags; uint64_t instructions; uint64_t host_page_size; uint64_t guest_page_size; uint64_t native_instructions; };
static_assert(sizeof(Probe) == 40 && alignof(Probe) == 8);
static_assert(offsetof(Probe, instructions) == 8 && offsetof(Probe, native_instructions) == 32);
extern "C" int32_t trames_android_probe(Probe*);

extern "C" JNIEXPORT jstring JNICALL Java_org_trames_android_NativeBridge_cpuProbe(JNIEnv* env, jclass) {
    Probe probe{};
    const int status = trames_android_probe(&probe);
    char report[512];
    std::snprintf(report, sizeof(report), "{\"status\":%d,\"schema\":%u,\"flags\":%u,\"instructions\":%llu,\"host_page_size\":%llu,\"guest_page_size\":%llu,\"native_instructions\":%llu}", status, probe.schema, probe.flags, (unsigned long long)probe.instructions, (unsigned long long)probe.host_page_size, (unsigned long long)probe.guest_page_size, (unsigned long long)probe.native_instructions);
    return env->NewStringUTF(report);
}
