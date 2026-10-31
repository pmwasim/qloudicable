package com.qloudihub.qloudicable.plugin

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbManager
import android.os.BatteryManager
import android.os.Build
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/** Collects what Android lets a normal app see. Rust does all the interpreting. */
@TauriPlugin
class QloudiCablePlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun facts(invoke: Invoke) {
        val out = JSObject()
        out.put("sdkInt", Build.VERSION.SDK_INT)

        // A sticky broadcast: registering with no receiver returns the latest state.
        val battery: Intent? = activity.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
        if (battery != null) {
            out.put("plugged", when (battery.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0)) {
                BatteryManager.BATTERY_PLUGGED_AC -> "ac"
                BatteryManager.BATTERY_PLUGGED_USB -> "usb"
                BatteryManager.BATTERY_PLUGGED_WIRELESS -> "wireless"
                8 -> "dock"
                else -> "none"
            })
            out.put("status", when (battery.getIntExtra(BatteryManager.EXTRA_STATUS, -1)) {
                BatteryManager.BATTERY_STATUS_CHARGING -> "charging"
                BatteryManager.BATTERY_STATUS_DISCHARGING -> "discharging"
                BatteryManager.BATTERY_STATUS_FULL -> "full"
                BatteryManager.BATTERY_STATUS_NOT_CHARGING -> "not_charging"
                else -> "unknown"
            })
            val mv = battery.getIntExtra(BatteryManager.EXTRA_VOLTAGE, -1)
            if (mv > 0) out.put("voltageMv", mv)
        }

        val bm = activity.getSystemService(Context.BATTERY_SERVICE) as BatteryManager
        val current = bm.getLongProperty(BatteryManager.BATTERY_PROPERTY_CURRENT_NOW)
        if (current != Long.MIN_VALUE) out.put("currentUa", current)
        val percent = bm.getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY)
        if (percent in 0..100) out.put("batteryPercent", percent)

        val devices = JSArray()
        val usb = activity.getSystemService(Context.USB_SERVICE) as UsbManager
        for (d in usb.deviceList.values) {
            val o = JSObject()
            o.put("vendorId", d.vendorId)
            o.put("productId", d.productId)
            o.put("deviceClass", d.deviceClass)
            // Newer Android hides these names until the user grants device access.
            runCatching { d.manufacturerName }.getOrNull()?.let { o.put("manufacturer", it) }
            runCatching { d.productName }.getOrNull()?.let { o.put("product", it) }
            runCatching { d.version }.getOrNull()?.let { o.put("version", it) }
            devices.put(o)
        }
        out.put("usbDevices", devices)
        invoke.resolve(out)
    }
}
