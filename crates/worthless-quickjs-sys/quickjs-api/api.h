#include "../quickjs/quickjs.h"

void WL_JS_FreeValue(JSContext *ctx, JSValue val);
JSValue WL_JS_NewFloat64(JSContext *ctx, double d);
JSValue WL_JS_NewInt32(JSContext *ctx, int32_t val);
JSValue WL_JS_NewBool(JSContext *ctx, JS_BOOL val);

const JSValue WL_JS_NULL;
const JSValue WL_JS_UNDEFINED;
const JSValue WL_JS_TRUE;