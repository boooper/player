package com.madrify.player

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.drawable.Icon
import android.media.MediaMetadata
import android.media.session.MediaSession
import android.media.session.PlaybackState
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import java.net.URL

class MediaPlaybackService : Service() {

    companion object {
        private const val CHANNEL_ID  = "madrify_media"
        private const val NOTIFICATION_ID = 42

        private const val ACTION_UPDATE    = "com.madrify.player.UPDATE"
        private const val ACTION_PLAY      = "com.madrify.player.PLAY"
        private const val ACTION_PAUSE     = "com.madrify.player.PAUSE"
        private const val ACTION_NEXT      = "com.madrify.player.NEXT"
        private const val ACTION_PREVIOUS  = "com.madrify.player.PREVIOUS"
        private const val ACTION_STOP      = "com.madrify.player.STOP"

        @JvmStatic
        fun update(
            ctx: Context,
            title: String, artist: String, album: String, coverUrl: String,
            isPlaying: Boolean, durationMs: Long, positionMs: Long,
        ) {
            val intent = Intent(ctx, MediaPlaybackService::class.java).apply {
                action = ACTION_UPDATE
                putExtra("title",      title)
                putExtra("artist",     artist)
                putExtra("album",      album)
                putExtra("coverUrl",   coverUrl)
                putExtra("isPlaying",  isPlaying)
                putExtra("durationMs", durationMs)
                putExtra("positionMs", positionMs)
            }
            ctx.startForegroundService(intent)
        }

        @JvmStatic
        fun setPlayingState(ctx: Context, isPlaying: Boolean, positionMs: Long) {
            val action = if (isPlaying) ACTION_PLAY else ACTION_PAUSE
            val intent = Intent(ctx, MediaPlaybackService::class.java).apply {
                this.action = action
                putExtra("positionMs", positionMs)
            }
            ctx.startForegroundService(intent)
        }

        @JvmStatic
        fun stop(ctx: Context) {
            ctx.stopService(Intent(ctx, MediaPlaybackService::class.java))
        }
    }

    // ── JNI callbacks – implemented in Rust (android_media.rs) ───────────────

    private external fun nativeOnPlay()
    private external fun nativeOnPause()
    private external fun nativeOnNext()
    private external fun nativeOnPrevious()
    private external fun nativeOnSeek(positionMs: Long)

    // ── State ─────────────────────────────────────────────────────────────────

    private var mediaSession: MediaSession? = null
    private val mainHandler = Handler(Looper.getMainLooper())

    private var title      = ""
    private var artist     = ""
    private var album      = ""
    private var coverUrl   = ""
    private var isPlaying  = false
    private var durationMs = 0L
    private var positionMs = 0L
    private var coverBitmap: Bitmap? = null

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        setupMediaSession()
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID, "Now Playing", NotificationManager.IMPORTANCE_LOW
        ).apply {
            description         = "Shows the currently playing track"
            setShowBadge(false)
            lockscreenVisibility = Notification.VISIBILITY_PUBLIC
        }
        getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    private fun setupMediaSession() {
        mediaSession = MediaSession(this, "MadrifyPlayer").apply {
            setCallback(object : MediaSession.Callback() {
                override fun onPlay()                { nativeOnPlay() }
                override fun onPause()               { nativeOnPause() }
                override fun onSkipToNext()          { nativeOnNext() }
                override fun onSkipToPrevious()      { nativeOnPrevious() }
                override fun onSeekTo(pos: Long)     { nativeOnSeek(pos) }
                override fun onStop()                {
                    stopForeground(STOP_FOREGROUND_REMOVE)
                    stopSelf()
                }
            })
            isActive = true
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_UPDATE -> {
                val newCoverUrl = intent.getStringExtra("coverUrl") ?: ""
                title      = intent.getStringExtra("title")     ?: title
                artist     = intent.getStringExtra("artist")    ?: artist
                album      = intent.getStringExtra("album")     ?: album
                isPlaying  = intent.getBooleanExtra("isPlaying", isPlaying)
                durationMs = intent.getLongExtra("durationMs", durationMs)
                positionMs = intent.getLongExtra("positionMs", positionMs)

                if (newCoverUrl != coverUrl) {
                    coverUrl   = newCoverUrl
                    coverBitmap = null
                    if (coverUrl.isNotEmpty()) {
                        Thread {
                            try {
                                val bmp = BitmapFactory.decodeStream(URL(coverUrl).openStream())
                                mainHandler.post { coverBitmap = bmp; updateNotification() }
                            } catch (_: Exception) {}
                        }.start()
                    }
                }
                updateNotification()
            }

            ACTION_PLAY -> {
                isPlaying  = true
                positionMs = intent.getLongExtra("positionMs", positionMs)
                updateNotification()
            }

            ACTION_PAUSE -> {
                isPlaying  = false
                positionMs = intent.getLongExtra("positionMs", positionMs)
                updateNotification()
            }

            ACTION_NEXT     -> nativeOnNext()
            ACTION_PREVIOUS -> nativeOnPrevious()

            ACTION_STOP -> {
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
                return START_NOT_STICKY
            }
        }
        return START_STICKY
    }

    // ── Notification ──────────────────────────────────────────────────────────

    private fun updateNotification() {
        val session = mediaSession ?: return

        // MediaMetadata
        val meta = MediaMetadata.Builder()
            .putString(MediaMetadata.METADATA_KEY_TITLE,  title)
            .putString(MediaMetadata.METADATA_KEY_ARTIST, artist)
            .putString(MediaMetadata.METADATA_KEY_ALBUM,  album)
            .putLong(MediaMetadata.METADATA_KEY_DURATION, durationMs)
            .apply { coverBitmap?.let { putBitmap(MediaMetadata.METADATA_KEY_ALBUM_ART, it) } }
            .build()
        session.setMetadata(meta)

        // PlaybackState
        val pbState = PlaybackState.Builder()
            .setActions(
                PlaybackState.ACTION_PLAY             or
                PlaybackState.ACTION_PAUSE            or
                PlaybackState.ACTION_SKIP_TO_NEXT     or
                PlaybackState.ACTION_SKIP_TO_PREVIOUS or
                PlaybackState.ACTION_SEEK_TO          or
                PlaybackState.ACTION_STOP
            )
            .setState(
                if (isPlaying) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED,
                positionMs,
                if (isPlaying) 1f else 0f,
            )
            .build()
        session.setPlaybackState(pbState)

        // PendingIntents
        val launchIntent = packageManager.getLaunchIntentForPackage(packageName)
        val contentPi = launchIntent?.let {
            PendingIntent.getActivity(this, 0, it,
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        }

        fun servicePi(action: String, requestCode: Int) = PendingIntent.getService(
            this, requestCode,
            Intent(this, MediaPlaybackService::class.java).setAction(action),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )

        // Notification
        val notification = Notification.Builder(this, CHANNEL_ID)
            .setContentTitle(title)
            .setContentText(artist)
            .setSubText(album.takeIf { it.isNotEmpty() })
            .setSmallIcon(R.drawable.ic_launcher_foreground)
            .setLargeIcon(coverBitmap)
            .setContentIntent(contentPi)
            .setVisibility(Notification.VISIBILITY_PUBLIC)
            .setOngoing(isPlaying)
            .addAction(Notification.Action.Builder(
                Icon.createWithResource(this, R.drawable.ic_media_prev),
                "Previous", servicePi(ACTION_PREVIOUS, 1),
            ).build())
            .addAction(Notification.Action.Builder(
                Icon.createWithResource(this,
                    if (isPlaying) R.drawable.ic_media_pause else R.drawable.ic_media_play),
                if (isPlaying) "Pause" else "Play",
                servicePi(if (isPlaying) ACTION_PAUSE else ACTION_PLAY, 2),
            ).build())
            .addAction(Notification.Action.Builder(
                Icon.createWithResource(this, R.drawable.ic_media_next),
                "Next", servicePi(ACTION_NEXT, 3),
            ).build())
            .setStyle(Notification.MediaStyle()
                .setMediaSession(session.sessionToken)
                .setShowActionsInCompactView(0, 1, 2))
            .build()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIFICATION_ID, notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        mediaSession?.release()
        mediaSession = null
        super.onDestroy()
    }
}
