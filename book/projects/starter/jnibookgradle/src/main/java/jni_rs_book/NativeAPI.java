package jni_rs_book;

// Note there can be many classes containing native methods, that there is no
// requirement that you name anything NativeAPI, and that libraries don't need
// to be loaded in static blocks.
class NativeAPI {

    private static final Throwable INIT_ERROR;

    // The static block will be executed the first time the NativeAPI
    // class is used.
    static {
        Throwable error = null;
        try {
            System.loadLibrary("jnibookrs");
        } catch (Throwable t) {
            error = t;
        }
        INIT_ERROR = error;
    }

    private NativeAPI() {
        // Not instantiable
    }

    static void verifyLink() {
        checkAvailability();
        verify_link();
    }

    static native int verify_link();

    static void checkAvailability() {
        if (INIT_ERROR != null) {
            if (INIT_ERROR instanceof RuntimeException) {
                throw (RuntimeException) INIT_ERROR;
            } else if (INIT_ERROR instanceof Error) {
                throw (Error) INIT_ERROR;
            } else {
                throw new RuntimeException(INIT_ERROR);
            }
        }
    }
}
