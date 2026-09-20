package com.goodboy008.labelize.demo

import android.app.Activity
import android.graphics.BitmapFactory
import android.os.Bundle
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import com.goodboy008.labelize.Labelize
import kotlin.concurrent.thread

/**
 * Minimal end-to-end check: renders a ZPL label through the JNI binding,
 * shows the PNG, and persists it (and a PDF) so `adb pull` can compare the
 * bytes against a desktop render of the same label.
 */
class MainActivity : Activity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val layout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(32, 32, 32, 32)
        }
        val status = TextView(this)
        val image = ImageView(this)
        layout.addView(status)
        layout.addView(image)
        setContentView(layout)

        val zpl = """
            ^XA
            ^FO50,50^A0N,44,44^FDHELLO FROM LABELIZE^FS
            ^FO50,130^A0N,28,28^FDRendered on Android^FS
            ^FO50,200^BCN,150,Y,N,N^FDABC-12345678^FS
            ^FO50,420^BQN,2,6^FDQA,https://github.com/GOODBOY008/labelize^FS
            ^XZ
        """.trimIndent().toByteArray()

        thread {
            try {
                val png = Labelize.renderZplToPng(zpl, widthMm = 102.0, heightMm = 152.0)
                val pdf = Labelize.renderZplToPdf(zpl, widthMm = 102.0, heightMm = 152.0)
                openFileOutput("render.png", MODE_PRIVATE).use { it.write(png) }
                openFileOutput("render.pdf", MODE_PRIVATE).use { it.write(pdf) }
                val bmp = BitmapFactory.decodeByteArray(png, 0, png.size)
                runOnUiThread {
                    status.text = "version: ${Labelize.version()}\n" +
                        "png: ${png.size} bytes ${bmp.width}x${bmp.height}\n" +
                        "pdf: ${pdf.size} bytes (${pdf.sliceArray(0..3).decodeToString()})"
                    image.setImageBitmap(bmp)
                }
            } catch (e: Exception) {
                runOnUiThread { status.text = "FAILED: $e" }
            }
        }
    }
}
