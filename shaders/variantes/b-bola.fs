#version 330

// "Tela empujada por una bola": la pantalla es una tela estirada y por debajo se empuja una bola
// de cristal. No hay borde de lente: la superficie se abomba suavemente, el contenido se estira en
// el centro y se comprime en la ladera, y la luz se nota en la pendiente (mas clara hacia la luz,
// mas oscura del lado contrario). Todo se funde con el resto de la pantalla.
in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D texture0;
uniform vec4 colDiffuse;

uniform vec2 cursorPosition;   // en coordenadas de gl_FragCoord (origen abajo-izquierda)
uniform float lensRadius;      // radio de influencia del bulto en pixeles (0 = sin efecto)
uniform float lensMag;         // aumento en el centro (1 = ninguno)

out vec4 finalColor;

// ─── AJUSTES (tocar aqui) ────────────────────────────────────────────────────
// Forma del bulto
const float HEIGHT          = 1.0;   // altura de la bola bajo la tela (mas = pendiente mas empinada, mas sombreado)
const float MAG_SHAPE       = 1.7;   // reparto del estiramiento: 1 = gradual (cono), mas alto = parejo en casi toda la cara y compresion solo cerca del borde (bola)
const float BALL            = 0.65;   // radio (0-1) hasta donde llega la parte esferica; despues la tela vuelve a plano
// Luz (de donde viene: x derecha, y arriba, z hacia ti)
const vec3  LIGHT_DIR       = vec3(-0.55, 0.65, 0.55);
const float SHADE_STRENGTH  = 0.3;   // cuanto se nota la luz/sombra en la pendiente (0 = nada)
const float SHEEN_STRENGTH  = 0.05;   // brillo suave de la tela en el lado de la luz (0 = mate)
const float SHEEN_POWER     = 10.0;   // mas = brillo mas concentrado
const float FOLD_SHADOW     = 0.07;   // sombra suave donde la tela se dobla al pie del bulto
// ─────────────────────────────────────────────────────────────────────────────

// Suavizado de 5.o grado (derivada y 2.a derivada nulas en 0 y 1): sin quiebres en el borde del efecto
float smoother(float t)  { return t * t * t * (t * (6.0 * t - 15.0) + 10.0); }
float dsmoother(float t) { return 30.0 * t * t * (1.0 - t) * (1.0 - t); }

void main()
{
    vec2 p = gl_FragCoord.xy;
    vec2 d = p - cursorPosition;
    float r = length(d);
    float R = lensRadius;

    vec4 base = texture(texture0, fragTexCoord);

    // Fuera del bulto la pantalla queda exactamente igual
    if (R < 1.0 || r >= R) {
        finalColor = base * colDiffuse;
        return;
    }

    float t = r / R;
    float s = smoother(pow(t, MAG_SHAPE));

    // Cuanto cambia la coordenada de textura por pixel (para convertir pixeles a coordenadas de textura)
    vec2 ddx = dFdx(fragTexCoord);
    vec2 ddy = dFdy(fragTexCoord);

    // Deformacion: el punto de la tela que se ve en r es el que estaba en r*k (k < 1 = estirado)
    float a = 1.0 / lensMag;
    float k = a + (1.0 - a) * s;
    vec2 off = -d * (1.0 - k);
    vec2 uv = fragTexCoord + off.x * ddx + off.y * ddy;
    vec3 col = texture(texture0, uv).rgb;

    // Iluminacion: normal de la superficie del bulto, altura h(t) = HEIGHT * (1 - s) * R
    vec2 dir = d / max(r, 1.0);
    float slope;
    if (t < BALL) { slope = t / sqrt(max(1.0 - t * t, 0.15)); }                 // casquete esferico: la pendiente crece hacia el borde de la bola
    else {
        float st = BALL / sqrt(max(1.0 - BALL * BALL, 0.15));
        slope = st * (1.0 - smoother((t - BALL) / (1.0 - BALL)));                // la tela vuelve a plano suavemente
    }
    slope *= HEIGHT;
    vec3 n = normalize(vec3(dir * slope, 1.0));      // la pendiente baja hacia afuera: la normal apunta hacia afuera
    vec3 L = normalize(LIGHT_DIR);
    float shade = 1.0 + (dot(n, L) - L.z) * SHADE_STRENGTH;   // 1.0 = tela plana; >1 hacia la luz, <1 al lado contrario
    col *= clamp(shade, 0.35, 1.5);

    // Brillo suave de la tela solo relativo a la tela plana
    vec3 hv = normalize(L + vec3(0.0, 0.0, 1.0));
    float spec = pow(max(dot(n, hv), 0.0), SHEEN_POWER) - pow(max(hv.z, 0.0), SHEEN_POWER);
    col += vec3(max(spec, 0.0) * SHEEN_STRENGTH);

    // Sombra suave donde la tela se dobla al pie del bulto
    col *= 1.0 - FOLD_SHADOW * smoothstep(0.55, 0.85, t) * (1.0 - smoothstep(0.85, 1.0, t));

    finalColor = vec4(clamp(col, 0.0, 1.0), base.a) * colDiffuse;
}
