uniform sampler2D font_tex;

in vec_2 f_tex_pos;
in vec_4 f_color;

out vec4 out_color;

void main(){
    float alpha= texture(font_tex, f_tex_pos).r;

    if(alpha <= 0.0){
        discard;
    }

    out_color = f_color * vec4(1.0, 1.0, 1.0, alpha);
}
