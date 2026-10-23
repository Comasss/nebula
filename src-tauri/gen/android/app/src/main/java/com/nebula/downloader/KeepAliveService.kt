package com.nebula.downloader

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat

/**
 * Servicio en primer plano mientras hay descargas: evita que Android congele o cierre Nebula
 * con la pantalla apagada o la app en segundo plano, y mantiene la CPU y el Wi-Fi despiertos.
 * Lo enciende y apaga el motor (NebulaPlugin.keepAlive) según haya descargas activas.
 */
class KeepAliveService : Service() {
  private var wakeLock: PowerManager.WakeLock? = null
  private var wifiLock: WifiManager.WifiLock? = null

  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val text = intent?.getStringExtra(EXTRA_TEXT) ?: ""
    val type = if (Build.VERSION.SDK_INT >= 34) ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE else 0
    try {
      ServiceCompat.startForeground(this, NOTIFICATION_ID, build(this, text), type)
    } catch (e: Exception) {
      stopSelf()
      return START_NOT_STICKY
    }
    acquireLocks()
    return START_NOT_STICKY
  }

  private fun acquireLocks() {
    if (wakeLock == null) {
      val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
      wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "nebula:downloads").apply {
        setReferenceCounted(false)
        acquire()
      }
    }
    if (wifiLock == null) {
      val wm = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
      @Suppress("DEPRECATION")
      val mode = if (Build.VERSION.SDK_INT >= 29) WifiManager.WIFI_MODE_FULL_LOW_LATENCY else WifiManager.WIFI_MODE_FULL_HIGH_PERF
      wifiLock = wm.createWifiLock(mode, "nebula:downloads").apply {
        setReferenceCounted(false)
        acquire()
      }
    }
  }

  override fun onDestroy() {
    wakeLock?.let { if (it.isHeld) it.release() }
    wifiLock?.let { if (it.isHeld) it.release() }
    wakeLock = null
    wifiLock = null
    super.onDestroy()
  }

  override fun onTimeout(startId: Int, fgsType: Int) {
    stopSelf()
  }

  companion object {
    const val CHANNEL_ID = "downloads"
    const val NOTIFICATION_ID = 1
    const val EXTRA_TEXT = "text"

    fun channels(context: Context) {
      if (Build.VERSION.SDK_INT < 26) return
      val nm = context.getSystemService(NotificationManager::class.java)
      nm.createNotificationChannel(
        NotificationChannel(CHANNEL_ID, context.getString(R.string.channel_downloads), NotificationManager.IMPORTANCE_LOW).apply {
          setShowBadge(false)
        }
      )
      nm.createNotificationChannel(
        NotificationChannel(NebulaPlugin.EVENTS_CHANNEL, context.getString(R.string.channel_events), NotificationManager.IMPORTANCE_DEFAULT)
      )
    }

    fun openAppIntent(context: Context): PendingIntent {
      val intent = Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_NEW_TASK)
      return PendingIntent.getActivity(context, 0, intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
    }

    fun build(context: Context, text: String): Notification {
      channels(context)
      return NotificationCompat.Builder(context, CHANNEL_ID)
        .setSmallIcon(R.drawable.ic_stat_nebula)
        .setColor(0xFF7C5CFF.toInt())
        .setContentTitle("Nebula")
        .setContentText(text)
        .setOngoing(true)
        .setOnlyAlertOnce(true)
        .setSilent(true)
        .setContentIntent(openAppIntent(context))
        .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
        .build()
    }
  }
}
