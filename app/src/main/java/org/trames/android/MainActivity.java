package org.trames.android;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;
import org.json.JSONObject;

public final class MainActivity extends Activity {
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        TextView view = new TextView(this);
        int padding = (int) (24 * getResources().getDisplayMetrics().density);
        view.setPadding(padding, padding, padding, padding);
        view.setText("Trames\n\n正在检查执行引擎…");
        setContentView(view);
        new Thread(() -> {
            String text;
            try {
                JSONObject report = new JSONObject(NativeBridge.cpuProbe());
                boolean ok = report.getInt("status") == 0;
                text = "Trames\n\n" + (ok ? "执行引擎检查通过" : "执行引擎检查失败")
                    + "\n\nAndroid 版本正在开发中，Linux 环境暂未接入。";
            } catch (Exception failure) { text = "Trames\n\n执行引擎检查失败"; }
            final String result = text;
            runOnUiThread(() -> view.setText(result));
        }, "TramesCpuProbe").start();
    }
}
