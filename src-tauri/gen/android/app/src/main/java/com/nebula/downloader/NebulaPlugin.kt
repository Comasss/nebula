package com.nebula.downloader

import android.Manifest
import android.app.Activity
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import android.provider.Settings
import android.webkit.MimeTypeMap
import android.view.HapticFeedbackConstants
import android.webkit.WebView
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

@InvokeArg
class NotifyArgs {
  var title: String = ""
  var body: String = ""
}

@InvokeArg
class KeepAliveArgs {
  var active: Boolean = false
  var text: String = ""
}

@InvokeArg
class PathArgs {
  var path: String = ""
}

@InvokeArg
class UrlArgs {
  var url: String = ""
}

@InvokeArg
class HapticArgs {
  var kind: String = "tap"
}

@InvokeArg
class ResumeArgs {
  var clipboard: Boolean = false
}

/**
 * Puente entre el motor de Nebula (Rust) y Android. Lo registra src-tauri/src/android.rs y se
 * llama desde allí; la interfaz web no lo usa directamente.
 */
@TauriPlugin
class NebulaPlugin(private val activity: Activity) : Plugin(activity) {
  private val pendingFiles = mutableListOf<String>()
  private val pendingUrls = mutableListOf<String>()
  private var notificationId = 100
  // La actividad puede recrearse; para lo que no necesita una ventana se usa la aplicación.
  private val ctx: Context get() = activity.applicationContext

  override fun load(webView: WebView) {
    KeepAliveService.channels(ctx)
    takeIntent(activity.intent)
  }

  override fun onNewIntent(intent: Intent) {
    takeIntent(intent)
  }

  /** .nzb abiertos con Nebula o compartidos con ella (archivos o enlaces). */
  private fun takeIntent(intent: Intent?) {
    if (intent == null) return
    val uris = mutableListOf<Uri>()
    when (intent.action) {
      Intent.ACTION_VIEW -> intent.data?.let { uris.add(it) }
      Intent.ACTION_SEND -> {
        @Suppress("DEPRECATION")
        (intent.getParcelableExtra<Uri>(Intent.EXTRA_STREAM))?.let { uris.add(it) }
        intent.getStringExtra(Intent.EXTRA_TEXT)?.let { text ->
          Regex("https?://\\S+").findAll(text).forEach { pendingUrls.add(it.value) }
        }
      }
      Intent.ACTION_SEND_MULTIPLE -> {
        @Suppress("DEPRECATION")
        intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM)?.let { uris.addAll(it) }
      }
      else -> return
    }
    for (uri in uris) {
      if (uri.scheme == "http" || uri.scheme == "https") {
        pendingUrls.add(uri.toString())
        continue
      }
      copyIn(uri)?.let { pendingFiles.add(it) }
    }
    // Que no se vuelva a procesar al recrear la actividad.
    intent.action = Intent.ACTION_MAIN
  }

  private fun copyIn(uri: Uri): String? {
    return try {
      var name = "nebula.nzb"
      if (uri.scheme == "content") {
        ctx.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
          if (c.moveToFirst() && !c.isNull(0)) name = c.getString(0)
        }
      } else {
        uri.lastPathSegment?.let { name = it.substringAfterLast('/') }
      }
      name = name.replace(Regex("[\\\\/:*?\"<>|]"), "_")
      val dir = File(ctx.filesDir, "incoming").apply { mkdirs() }
      val out = File(dir, name)
      ctx.contentResolver.openInputStream(uri)?.use { input ->
        out.outputStream().use { input.copyTo(it) }
      } ?: return null
      out.absolutePath
    } catch (e: Exception) {
      null
    }
  }

  @Command
  fun resume(invoke: Invoke) {
    val args = invoke.parseArgs(ResumeArgs::class.java)
    val res = JSObject()
    val files = JSArray()
    pendingFiles.forEach { files.put(it) }
    pendingFiles.clear()
    val urls = JSArray()
    pendingUrls.forEach { urls.put(it) }
    pendingUrls.clear()
    res.put("files", files)
    res.put("urls", urls)
    var clip = ""
    if (args.clipboard) {
      try {
        val cm = ctx.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val item = cm.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)
        clip = item?.coerceToText(ctx)?.toString() ?: ""
      } catch (_: Exception) {
      }
    }
    res.put("clipboard", clip)
    invoke.resolve(res)
  }

  @Command
  fun showNotification(invoke: Invoke) {
    val args = invoke.parseArgs(NotifyArgs::class.java)
    try {
      val n = NotificationCompat.Builder(ctx, EVENTS_CHANNEL)
        .setSmallIcon(R.drawable.ic_stat_nebula)
        .setColor(0xFF7C5CFF.toInt())
        .setContentTitle(args.title)
        .setContentText(args.body)
        .setStyle(NotificationCompat.BigTextStyle().bigText(args.body))
        .setAutoCancel(true)
        .setContentIntent(KeepAliveService.openAppIntent(ctx))
        .build()
      NotificationManagerCompat.from(ctx).notify(notificationId++, n)
    } catch (_: SecurityException) {
      // Sin permiso de notificaciones: no se avisa.
    }
    invoke.resolve()
  }

  @Command
  fun keepAlive(invoke: Invoke) {
    val args = invoke.parseArgs(KeepAliveArgs::class.java)
    val intent = Intent(ctx, KeepAliveService::class.java)
    try {
      if (args.active) {
        intent.putExtra(KeepAliveService.EXTRA_TEXT, args.text)
        ContextCompat.startForegroundService(ctx, intent)
      } else {
        ctx.stopService(intent)
      }
      invoke.resolve()
    } catch (e: Exception) {
      // Android no deja iniciarlo desde segundo plano: se intentará al volver a la app.
      invoke.reject(e.message ?: "keepAlive")
    }
  }

  @Command
  fun openPath(invoke: Invoke) {
    val path = invoke.parseArgs(PathArgs::class.java).path
    var file = File(path)
    while (!file.exists() && file.parentFile != null) file = file.parentFile!!
    try {
      if (file.isDirectory) openFolder(file) else openFile(file)
      invoke.resolve()
    } catch (e: Exception) {
      invoke.reject(e.message ?: "openPath")
    }
  }

  private fun openFile(file: File) {
    val uri = FileProvider.getUriForFile(ctx, ctx.packageName + ".fileprovider", file)
    val ext = file.extension.lowercase()
    val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext) ?: "*/*"
    val intent = Intent(Intent.ACTION_VIEW).setDataAndType(uri, mime).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
    ctx.startActivity(Intent.createChooser(intent, null).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
  }

  /**
   * Abre la carpeta en un gestor de archivos. Si hay varios instalados, Android deja elegir con
   * cuál; si ninguno sabe abrir carpetas, se ofrece abrir el archivo principal de la descarga.
   */
  private fun openFolder(dir: File) {
    val root = Environment.getExternalStorageDirectory().absolutePath
    val rel = dir.absolutePath.removePrefix(root).trim('/')
    val docId = if (rel.isEmpty()) "primary:" else "primary:$rel"
    val docUri = DocumentsContract.buildDocumentUri("com.android.externalstorage.documents", docId)
    val ownUri = FileProvider.getUriForFile(ctx, ctx.packageName + ".fileprovider", dir)
    // Cada gestor entiende una forma distinta de "esta carpeta". El permiso de lectura solo se
    // puede conceder sobre las URIs propias: en las del almacenamiento del sistema Android lo
    // rechaza y no se abriría nada.
    val grant = Intent.FLAG_GRANT_READ_URI_PERMISSION
    val variants = listOf(
      Intent(Intent.ACTION_VIEW).setDataAndType(docUri, DocumentsContract.Document.MIME_TYPE_DIR),
      Intent(Intent.ACTION_VIEW).setDataAndType(docUri, "resource/folder"),
      Intent(Intent.ACTION_VIEW).setDataAndType(ownUri, "resource/folder").addFlags(grant),
      Intent(Intent.ACTION_VIEW).setDataAndType(ownUri, "inode/directory").addFlags(grant),
      Intent(Intent.ACTION_VIEW).setDataAndType(ownUri, DocumentsContract.Document.MIME_TYPE_DIR).addFlags(grant),
    )
    val pm = ctx.packageManager
    val seen = mutableSetOf(ctx.packageName)
    val targets = mutableListOf<Intent>()
    for (variant in variants) {
      for (info in pm.queryIntentActivities(variant, 0)) {
        val app = info.activityInfo
        if (seen.add(app.packageName)) targets += Intent(variant).setClassName(app.packageName, app.name)
      }
    }
    // Gestores que no anuncian abrir carpetas concretas: al menos se pueden abrir.
    if (Build.VERSION.SDK_INT >= 29) {
      for (info in pm.queryIntentActivities(Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_APP_FILES), 0)) {
        val app = info.activityInfo
        if (seen.add(app.packageName)) pm.getLaunchIntentForPackage(app.packageName)?.let { targets += it }
      }
    }
    if (targets.isEmpty()) {
      val main = dir.walkTopDown().filter { it.isFile }.maxByOrNull { it.length() } ?: throw Exception("No file manager")
      openFile(main)
      return
    }
    val first = targets.removeAt(0).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    if (targets.isEmpty()) {
      ctx.startActivity(first)
    } else {
      val chooser = Intent.createChooser(first, null)
        .putExtra(Intent.EXTRA_INITIAL_INTENTS, targets.toTypedArray())
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
      ctx.startActivity(chooser)
    }
  }

  /**
   * Abre el APK de una actualización con el instalador de Android. La primera vez Android pide
   * permitir que Nebula instale apps: se abre esa pantalla y la interfaz deja volver a intentarlo.
   */
  @Command
  fun installApk(invoke: Invoke) {
    val path = invoke.parseArgs(PathArgs::class.java).path
    val res = JSObject()
    try {
      if (Build.VERSION.SDK_INT >= 26 && !ctx.packageManager.canRequestPackageInstalls()) {
        val settings = Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + ctx.packageName))
        activity.startActivity(settings)
        res.put("started", false)
        res.put("needsPermission", true)
        invoke.resolve(res)
        return
      }
      val uri = FileProvider.getUriForFile(ctx, ctx.packageName + ".fileprovider", File(path))
      val intent = Intent(Intent.ACTION_VIEW)
        .setDataAndType(uri, "application/vnd.android.package-archive")
        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
      activity.startActivity(intent)
      res.put("started", true)
      res.put("needsPermission", false)
      invoke.resolve(res)
    } catch (e: Exception) {
      invoke.reject(e.message ?: "installApk")
    }
  }

  @Command
  fun moveToBack(invoke: Invoke) {
    activity.runOnUiThread { activity.moveTaskToBack(true) }
    invoke.resolve()
  }

  /** Vibración breve de respuesta al tocar (respeta el ajuste de vibración táctil del sistema). */
  @Command
  fun haptic(invoke: Invoke) {
    val kind = invoke.parseArgs(HapticArgs::class.java).kind
    val effect = when (kind) {
      "long" -> HapticFeedbackConstants.LONG_PRESS
      "tick" -> HapticFeedbackConstants.CLOCK_TICK
      "success" -> if (Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.CONFIRM else HapticFeedbackConstants.CONTEXT_CLICK
      "error" -> if (Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.REJECT else HapticFeedbackConstants.LONG_PRESS
      else -> HapticFeedbackConstants.KEYBOARD_TAP
    }
    activity.runOnUiThread { activity.window.decorView.performHapticFeedback(effect) }
    invoke.resolve()
  }

  @Command
  fun openUrl(invoke: Invoke) {
    val url = invoke.parseArgs(UrlArgs::class.java).url
    try {
      ctx.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
      invoke.resolve()
    } catch (e: Exception) {
      invoke.reject(e.message ?: "openUrl")
    }
  }

  private fun hasStorage(): Boolean {
    return if (Build.VERSION.SDK_INT >= 30) {
      Environment.isExternalStorageManager()
    } else {
      ContextCompat.checkSelfPermission(ctx, Manifest.permission.WRITE_EXTERNAL_STORAGE) == PackageManager.PERMISSION_GRANTED
    }
  }

  @Command
  fun storageAccess(invoke: Invoke) {
    invoke.resolve(JSObject().put("granted", hasStorage()))
  }

  /** Acceso a todos los archivos, para guardar donde se elija (Descargas, tarjeta SD…). */
  @Command
  fun requestStorageAccess(invoke: Invoke) {
    if (!hasStorage()) {
      try {
        if (Build.VERSION.SDK_INT >= 30) {
          val intent = Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION, Uri.parse("package:" + ctx.packageName))
          try {
            activity.startActivity(intent)
          } catch (_: Exception) {
            activity.startActivity(Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION))
          }
        } else {
          ActivityCompat.requestPermissions(
            activity,
            arrayOf(Manifest.permission.WRITE_EXTERNAL_STORAGE, Manifest.permission.READ_EXTERNAL_STORAGE),
            2
          )
        }
      } catch (_: Exception) {
      }
    }
    invoke.resolve(JSObject().put("granted", hasStorage()))
  }

  companion object {
    const val EVENTS_CHANNEL = "events"
  }
}
