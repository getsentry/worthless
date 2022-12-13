#include "../quickjs/quickjs.h"

JSValue WL_JS_NewFloat64(JSContext *ctx, double d);
JSValue WL_JS_NewInt32(JSContext *ctx, int32_t val);
JSValue WL_JS_NewBool(JSContext *ctx, JS_BOOL val);