package com.goodboy008.labelize

/**
 * Thrown when [Labelize] fails to parse or render a label.
 *
 * @param stage [Labelize.STAGE_PARSE] for malformed label data,
 *              [Labelize.STAGE_RENDER] for internal rendering errors.
 * @param message human-readable failure description.
 */
class LabelizeException(val stage: Int, message: String) : RuntimeException(message) {

    companion object {
        /** Name matching the value of [stage]. */
        fun stageName(stage: Int): String = when (stage) {
            Labelize.STAGE_PARSE -> "PARSE"
            Labelize.STAGE_RENDER -> "RENDER"
            else -> "UNKNOWN"
        }
    }
}
