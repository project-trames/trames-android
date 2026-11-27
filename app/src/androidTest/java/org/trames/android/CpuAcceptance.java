package org.trames.android;

import android.app.Instrumentation;
import android.app.Activity;
import android.os.Bundle;
import android.content.pm.ApplicationInfo;
import org.json.JSONObject;

/** Run on a real app UID and a non-debuggable validation APK. Shell/root CPU
 * probes and a successful cross-build are not device acceptance. */
public final class CpuAcceptance extends Instrumentation {
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            int uid = android.os.Process.myUid();
            boolean debuggable = (getTargetContext().getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) != 0;
            if (uid < 10000 || debuggable) throw new IllegalStateException("Requires ordinary app UID and non-debuggable target");
            String raw = NativeBridge.cpuProbe();
            JSONObject report = new JSONObject(raw);
            if (report.getInt("status") != 0 || report.getLong("native_instructions") != 2 || report.getLong("guest_page_size") != 4096 || report.getLong("instructions") != 3) throw new IllegalStateException(raw);
            result.putString("cpu_report", raw);
            result.putInt("app_uid", uid);
            result.putBoolean("debuggable", debuggable);
            finish(Activity.RESULT_OK, result);
        } catch (Exception failure) { result.putString("failure", failure.toString()); finish(Activity.RESULT_CANCELED, result); }
    }
}
