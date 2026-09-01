package fixtures;

import java.lang.invoke.MethodHandles;
import java.util.ServiceLoader;
import sun.misc.Unsafe;

@Deprecated
public final class StaticEvidence {
    @Deprecated private int value;
    private Unsafe internalApi;

    public native void nativeCall();

    public Class<?> loadStatic() throws Exception {
        return Class.forName("fixtures.Plugin");
    }

    public Class<?> loadDynamic(String name) throws Exception {
        return Class.forName(name);
    }

    @Deprecated public void evidence() {
        System.loadLibrary("fixture");
        StaticEvidence.class.getResourceAsStream("/fixture.txt");
        ServiceLoader.load(Runnable.class);
        MethodHandles.lookup();
        Runnable lambda = () -> value++;
        lambda.run();
    }
}
