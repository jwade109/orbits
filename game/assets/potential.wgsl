void mainImage(out vec4 fragColor, in vec2 fragCoord)
{
    vec2 p1 = vec2(0.4, 0.3) * iResolution.xy;
    vec2 p2 = p1 + vec2(0.1, 0.4) * iResolution.xy;
    vec2 p3 = p1 + vec2(0.2, 0.1) * iResolution.xy;

    float pot = 1.0 - 20.0 * (
        5.0 / length(p1 - fragCoord) +
        1.0 / length(p2 - fragCoord) +
        0.6 / length(p3 - fragCoord)
    );

    bool iso = false;
    for(int i = -20; i < 10; i += 1){
        float l1 = float(i) / 10.0;
        float l2 = l1 + 0.01;
        if (l1 < pot && pot < l2) {
            iso = true;
            break;
        }
    }

    if (length(p1 - fragCoord) < 10.0) {
        fragColor = vec4(vec3(1), 1);
    } else if (length(p2 - fragCoord) < 10.0) {
        fragColor = vec4(vec3(1), 1);
    } else if (iso) {
        fragColor = vec4(vec3(0.4), 1);
    } else {
        fragColor = vec4(vec3(0), 1);
    }
}
