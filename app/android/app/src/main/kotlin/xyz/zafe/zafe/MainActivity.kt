package xyz.zafe.zafe

import android.app.UiModeManager
import android.content.ActivityNotFoundException
import android.content.Intent
import android.os.Build
import android.provider.Settings
import android.view.WindowManager
import io.flutter.embedding.android.FlutterFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

// A FragmentActivity so local_auth can show the biometric / screen-lock prompt.
class MainActivity : FlutterFragmentActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        // Screens showing secrets (invites, backups) block screenshots, screen recording
        // and the recent-apps thumbnail while they are open.
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "xyz.zafe/secure_screen")
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "setSecure" -> {
                        val secure = call.arguments as? Boolean ?: false
                        runOnUiThread {
                            if (secure) {
                                window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
                            } else {
                                window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
                            }
                        }
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            }
        // The app's light/dark setting. The system splash is drawn before Flutter starts,
        // so it follows the OS theme unless the app's own night mode is set (Android 12+;
        // the OS persists it across launches). Older versions keep the OS theme.
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "xyz.zafe/window_appearance")
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "setBrightness" -> {
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                            val mode = when (call.argument<String>("brightness")) {
                                "dark" -> UiModeManager.MODE_NIGHT_YES
                                "light" -> UiModeManager.MODE_NIGHT_NO
                                else -> UiModeManager.MODE_NIGHT_AUTO
                            }
                            getSystemService(UiModeManager::class.java)
                                ?.setApplicationNightMode(mode)
                        }
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            }
        // Opens the system's security settings (to set a screen lock); falls back to the
        // main Settings screen where a vendor doesn't have that page.
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "xyz.zafe/system_settings")
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "openSecurity" -> {
                        val opened = listOf(
                            Settings.ACTION_SECURITY_SETTINGS,
                            Settings.ACTION_SETTINGS,
                        ).any { action ->
                            try {
                                startActivity(Intent(action))
                                true
                            } catch (e: ActivityNotFoundException) {
                                false
                            }
                        }
                        result.success(opened)
                    }
                    else -> result.notImplemented()
                }
            }
    }
}
