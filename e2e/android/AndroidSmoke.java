import com.goodboy008.labelize.Labelize;
import com.goodboy008.labelize.LabelizeException;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;

/** Host JVM check of the actual Kotlin/JNI boundary; no Android device is needed. */
public class AndroidSmoke {
    public static void main(String[] args) {
        byte[] source = ("^XA^FO20,20^A0N,30,20^FDPage One^FS^XZ"
                + "^XA^FO20,20^A0N,30,20^FDPage Two^FS^XZ").getBytes(StandardCharsets.UTF_8);
        byte[] png = Labelize.INSTANCE.renderOutput(source, 40, 30, 8, false, false, "zpl", "vector", 3, false);
        if (ByteBuffer.wrap(png).getInt(16) != 960 || ByteBuffer.wrap(png).getInt(20) != 720) {
            throw new AssertionError("PNG dimensions");
        }
        byte[] pdf = Labelize.INSTANCE.renderVectorPdf(source, "zpl", 40, 30, 8, true);
        if (!new String(pdf, 0, 4, StandardCharsets.US_ASCII).equals("%PDF")) {
            throw new AssertionError("PDF header");
        }
        try {
            Labelize.INSTANCE.renderOutput(source, 40, 30, 8, false, false, "zpl", "vector", Double.NaN, false);
            throw new AssertionError("invalid scale was accepted");
        } catch (LabelizeException e) {
            if (e.getStage() != Labelize.STAGE_PARSE) throw new AssertionError("incorrect error stage");
        }
        System.out.println("Kotlin/JNI vector PNG, PDF, multipage request, and invalid-option exception: PASS");
    }
}
