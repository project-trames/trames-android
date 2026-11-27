package org.trames.android;

public final class NativeBridge {
    static { System.loadLibrary("trames_android"); }
    private NativeBridge() {}
    public static native String cpuProbe();
}
