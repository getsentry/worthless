#include "api.h"

JSValue WL_JS_NewFloat64(JSContext *ctx, double d)
{
    return __JS_NewFloat64(ctx, d);
}

JSValue WL_JS_NewInt32(JSContext *ctx, int32_t val)
{
    return JS_NewInt32(ctx, val);
}

JSValue WL_JS_NewBool(JSContext *ctx, int32_t val)
{
    return JS_NewBool(ctx, val); 
}