package com.goodboy008.labelize

/**
 * Kotlin API over the labelize rendering engine (ZPL/EPL/TSPL → PNG/PDF).
 *
 * The native library is loaded once per process; all entry points are safe to
 * call from any thread. Failures surface as [LabelizeException] with a [stage]
 * distinguishing bad input (parse) from engine errors (render).
 */
object Labelize {
    /** Exception stage: the label data could not be parsed. */
    const val STAGE_PARSE: Int = 1

    /** Exception stage: the label parsed but could not be rendered. */
    const val STAGE_RENDER: Int = 2

    init {
        System.loadLibrary("labelize_android")
    }

    /**
     * Raw entry point, mirroring the JS API of `@goodboy008/labelize-wasm`.
     *
     * @param src   raw ZPL or EPL label bytes
     * @param widthMm  label width in mm (defines the output pixel width together with [dpmm])
     * @param heightMm label height in mm
     * @param dpmm  dot density in dots per mm (typically 6, 8, 12, or 24)
     * @param antialias enable grayscale anti-aliasing (off = 1-bit threshold, like the printers)
     * @param pdf   return a one-page PDF instead of a PNG
     * @param epl   interpret [src] as EPL instead of ZPL
     */
    external fun render(
        src: ByteArray,
        widthMm: Double,
        heightMm: Double,
        dpmm: Int,
        antialias: Boolean,
        pdf: Boolean,
        epl: Boolean,
    ): ByteArray

    /**
     * Language-explicit entry point: [lang] is `"zpl"`, `"epl"` or `"tspl"`.
     * Unknown values fall back to ZPL, matching the CLI's sniff fallback.
     */
    external fun renderLang(
        src: ByteArray,
        widthMm: Double,
        heightMm: Double,
        dpmm: Int,
        antialias: Boolean,
        pdf: Boolean,
        lang: String,
    ): ByteArray

    /** Engine version string, e.g. `labelize-android 0.1.0`. */
    external fun version(): String

    /** Render ZPL label data to a PNG image. */
    fun renderZplToPng(
        zpl: ByteArray,
        widthMm: Double = 102.0,
        heightMm: Double = 152.0,
        dpmm: Int = 8,
        antialias: Boolean = false,
    ): ByteArray = render(zpl, widthMm, heightMm, dpmm, antialias, pdf = false, epl = false)

    /** Render EPL label data to a PNG image. */
    fun renderEplToPng(
        epl: ByteArray,
        widthMm: Double = 102.0,
        heightMm: Double = 152.0,
        dpmm: Int = 8,
        antialias: Boolean = false,
    ): ByteArray = render(epl, widthMm, heightMm, dpmm, antialias, pdf = false, epl = true)

    /** Render ZPL label data to a one-page PDF. */
    fun renderZplToPdf(
        zpl: ByteArray,
        widthMm: Double = 102.0,
        heightMm: Double = 152.0,
        dpmm: Int = 8,
        antialias: Boolean = false,
    ): ByteArray = render(zpl, widthMm, heightMm, dpmm, antialias, pdf = true, epl = false)

    /** Render TSPL/TSC label data to a PNG image. The label's `SIZE` command
     * does not override [widthMm]/[heightMm] — size the canvas like any other input. */
    fun renderTsplToPng(
        tspl: ByteArray,
        widthMm: Double = 102.0,
        heightMm: Double = 152.0,
        dpmm: Int = 8,
        antialias: Boolean = false,
    ): ByteArray = renderLang(tspl, widthMm, heightMm, dpmm, antialias, pdf = false, lang = "tspl")

    /** Render CPCL (Zebra mobile) label data to a PNG. */
    fun renderCpclToPng(
        cpcl: ByteArray,
        widthMm: Double = 102.0,
        heightMm: Double = 152.0,
        dpmm: Int = 8,
        antialias: Boolean = false,
    ): ByteArray = renderLang(cpcl, widthMm, heightMm, dpmm, antialias, pdf = false, lang = "cpcl")

    /** Render CPCL (Zebra mobile) label data to a one-page PDF. */
    fun renderCpclToPdf(
        cpcl: ByteArray,
        widthMm: Double = 102.0,
        heightMm: Double = 152.0,
        dpmm: Int = 8,
        antialias: Boolean = false,
    ): ByteArray = renderLang(cpcl, widthMm, heightMm, dpmm, antialias, pdf = true, lang = "cpcl")

    /** Render DPL (Datamax-O'Neil) label data to a PNG. */
    fun renderDplToPng(
        dpl: ByteArray,
        widthMm: Double = 102.0,
        heightMm: Double = 152.0,
        dpmm: Int = 8,
        antialias: Boolean = false,
    ): ByteArray = renderLang(dpl, widthMm, heightMm, dpmm, antialias, pdf = false, lang = "dpl")

    /** Render DPL (Datamax-O'Neil) label data to a one-page PDF. */
    fun renderDplToPdf(
        dpl: ByteArray,
        widthMm: Double = 102.0,
        heightMm: Double = 152.0,
        dpmm: Int = 8,
        antialias: Boolean = false,
    ): ByteArray = renderLang(dpl, widthMm, heightMm, dpmm, antialias, pdf = true, lang = "dpl")

    /** Render TSPL/TSC label data to a one-page PDF. */
    fun renderTsplToPdf(
        tspl: ByteArray,
        widthMm: Double = 102.0,
        heightMm: Double = 152.0,
        dpmm: Int = 8,
        antialias: Boolean = false,
    ): ByteArray = renderLang(tspl, widthMm, heightMm, dpmm, antialias, pdf = true, lang = "tspl")

    /** Render EPL label data to a one-page PDF. */
    fun renderEplToPdf(
        epl: ByteArray,
        widthMm: Double = 102.0,
        heightMm: Double = 152.0,
        dpmm: Int = 8,
        antialias: Boolean = false,
    ): ByteArray = render(epl, widthMm, heightMm, dpmm, antialias, pdf = true, epl = true)
}
