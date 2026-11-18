#!/usr/bin/env python3
"""Compile production GL shaders and check rendered materials and surface grids.

Uses native CGL on macOS, or libEGL/libGL with a Mesa surfaceless adapter.
Missing GL support is a failure, never a passing skip. No Python packages are required.
"""
import ctypes as c
from pathlib import Path
import re
import sys

G = c.CDLL('/System/Library/Frameworks/OpenGL.framework/OpenGL' if sys.platform == 'darwin' else 'libGL.so.1')
E = None if sys.platform == 'darwin' else c.CDLL('libEGL.so.1')
I=c.c_int; U=c.c_uint; F=c.c_float; P=c.c_void_p

def api(lib, name, result, *arguments):
    function=getattr(lib,name);function.restype=result;function.argtypes=arguments;return function

def ints(*values):return (I*len(values))(*values)

def require(ok,message):
    if not ok:raise RuntimeError(message)

if sys.platform == 'darwin':
    pixel_format=P(); count=I(); context=P()
    require(api(G,'CGLChoosePixelFormat',I,c.POINTER(I),c.POINTER(P),c.POINTER(I))(
        ints(99,0x3200,8,32,12,24,0),c.byref(pixel_format),c.byref(count))==0,'CGL pixel format')
    require(api(G,'CGLCreateContext',I,P,P,c.POINTER(P))(pixel_format,None,c.byref(context))==0,'CGL context')
    require(api(G,'CGLSetCurrentContext',I,P)(context)==0,'CGL current context')
    api(G,'CGLDestroyPixelFormat',I,P)(pixel_format)
    framebuffer=U();api(G,'glGenFramebuffers',None,I,c.POINTER(U))(1,c.byref(framebuffer))
    api(G,'glBindFramebuffer',None,U,U)(0x8D40,framebuffer)
    for attachment,format in [(0x8CE0,0x8058),(0x8D00,0x81A6)]:
        buffer=U();api(G,'glGenRenderbuffers',None,I,c.POINTER(U))(1,c.byref(buffer))
        api(G,'glBindRenderbuffer',None,U,U)(0x8D41,buffer)
        api(G,'glRenderbufferStorage',None,U,U,I,I)(0x8D41,format,64,64)
        api(G,'glFramebufferRenderbuffer',None,U,U,U,U)(0x8D40,attachment,0x8D41,buffer)
    require(api(G,'glCheckFramebufferStatus',U,U)(0x8D40)==0x8CD5,'CGL framebuffer')
else:
    get_display=api(E,'eglGetPlatformDisplay',P,U,P,c.POINTER(I))
    display=get_display(0x31DD,None,None)
    require(api(E,'eglInitialize',U,P,c.POINTER(I),c.POINTER(I))(display,None,None),'EGL initialize')
    require(api(E,'eglBindAPI',U,U)(0x30A2),'OpenGL API')
    config=P();count=I()
    require(api(E,'eglChooseConfig',U,P,c.POINTER(I),c.POINTER(P),I,c.POINTER(I))(
        display,ints(0x3033,1,0x3040,8,0x3024,8,0x3023,8,0x3022,8,0x3021,8,0x3025,24,0x3038),c.byref(config),1,c.byref(count)) and count.value,'EGL config')
    surface=api(E,'eglCreatePbufferSurface',P,P,P,c.POINTER(I))(display,config,ints(0x3057,64,0x3056,64,0x3038))
    context=api(E,'eglCreateContext',P,P,P,P,c.POINTER(I))(display,config,None,ints(0x3098,3,0x30FB,2,0x30FD,1,0x3038))
    require(context and surface,'EGL context/surface')
    require(api(E,'eglMakeCurrent',U,P,P,P,P)(display,surface,surface,context),'EGL make current')

source=(Path(__file__).resolve().parents[2]/'crates/render/src/shaders.rs').read_text()
def shader_text(name):
    text=re.search(r'pub const '+name+r': &str = r#"(.*?)"#;',source,re.S).group(1)
    text = text.replace('// SURFACE_GRID', (Path(__file__).resolve().parents[2]/'crates/render/src/surface_grid.glsl').read_text())
    return text.replace('// NORMAL_MAPPING',re.search(r'pub const NORMAL_MAPPING: &str = r#"(.*?)"#;',source,re.S).group(1))

def compile_shader(name,kind):
    shader=api(G,'glCreateShader',U,U)(kind);encoded=shader_text(name).encode();pointer=c.c_char_p(encoded)
    api(G,'glShaderSource',None,U,I,c.POINTER(c.c_char_p),c.POINTER(I))(shader,1,c.byref(pointer),None)
    api(G,'glCompileShader',None,U)(shader);status=I()
    api(G,'glGetShaderiv',None,U,U,c.POINTER(I))(shader,0x8B81,c.byref(status))
    log=c.create_string_buffer(32768);api(G,'glGetShaderInfoLog',None,U,I,c.POINTER(I),P)(shader,len(log),None,log)
    require(status.value,name+': '+log.value.decode());return shader

def program(prefix):
    result=api(G,'glCreateProgram',U)()
    for suffix,kind in [('VERTEX',0x8B31),('FRAGMENT',0x8B30)]:
        api(G,'glAttachShader',None,U,U)(result,compile_shader(prefix+'_'+suffix,kind))
    api(G,'glBindFragDataLocation',None,U,U,c.c_char_p)(result,0,b'fragment')
    api(G,'glLinkProgram',None,U)(result);status=I()
    api(G,'glGetProgramiv',None,U,U,c.POINTER(I))(result,0x8B82,c.byref(status))
    log=c.create_string_buffer(32768);api(G,'glGetProgramInfoLog',None,U,I,c.POINTER(I),P)(result,len(log),None,log)
    require(status.value,prefix+' link: '+log.value.decode());return result

try:
    print('OpenGL:', api(G,'glGetString',c.c_char_p,U)(0x1F01).decode(),
          api(G,'glGetString',c.c_char_p,U)(0x1F02).decode())
    scene=program('SCENE');normal=program('NORMAL')
    compile_shader('COMPOSITE_FRAGMENT',0x8B30)
    api(G,'glUseProgram',None,U)(normal)
    location=api(G,'glGetUniformLocation',I,U,c.c_char_p)
    def uniform(name,value):api(G,'glUniform1f',None,I,F)(location(normal,name.encode()),value)
    identity=(F*16)(1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1)
    for name in ['view','projection']:
        api(G,'glUniformMatrix4fv',None,I,I,U,c.POINTER(F))(location(normal,name.encode()),1,0,identity)
    vao=U();api(G,'glGenVertexArrays',None,I,c.POINTER(U))(1,c.byref(vao));api(G,'glBindVertexArray',None,U)(vao)
    buffer=U();api(G,'glGenBuffers',None,I,c.POINTER(U))(1,c.byref(buffer));api(G,'glBindBuffer',None,U,U)(0x8892,buffer)
    vertices=(F*24)(-.8,-.8,0,0,0,1,0,0, .8,-.8,0,0,0,1,1,0, 0,.8,0,0,0,1,.5,1)
    api(G,'glBufferData',None,U,c.c_ssize_t,P,U)(0x8892,c.sizeof(vertices),vertices,0x88E4)
    for name,size,offset in [('position',3,0),('normal',3,12),('uv',2,24)]:
        index=api(G,'glGetAttribLocation',I,U,c.c_char_p)(normal,name.encode());require(index>=0,name+' attribute')
        api(G,'glEnableVertexAttribArray',None,U)(index)
        api(G,'glVertexAttribPointer',None,U,I,U,U,I,P)(index,size,0x1406,0,32,P(offset))
    texture=U();api(G,'glGenTextures',None,I,c.POINTER(U))(1,c.byref(texture));api(G,'glBindTexture',None,U,U)(0x0DE1,texture)
    pixel=(c.c_ubyte*4)(191,128,238,255)
    api(G,'glTexImage2D',None,U,I,I,I,I,I,U,U,P)(0x0DE1,0,0x8058,1,1,0,0x1908,0x1401,pixel)
    for parameter in [0x2800,0x2801]:api(G,'glTexParameteri',None,U,U,I)(0x0DE1,parameter,0x2600)
    api(G,'glUniform1i',None,I,I)(location(normal,b'normal_map'),0)
    api(G,'glUniform2f',None,I,F,F)(location(normal,b'material_size'),1,1)
    uniform('normal_scale',1);uniform('normal_y_sign',1)
    api(G,'glViewport',None,I,I,I,I)(0,0,64,64)
    def render(mode,back,mapped):
        uniform('material_face_mode',mode);uniform('has_normal_map',mapped)
        api(G,'glFrontFace',None,U)(0x0900 if back else 0x0901)
        api(G,'glClearColor',None,F,F,F,F)(0,0,0,0);api(G,'glClear',None,U)(0x4000)
        api(G,'glDrawArrays',None,U,I,I)(4,0,3)
        rgba=(c.c_ubyte*4)();api(G,'glReadPixels',None,I,I,I,I,U,U,P)(32,24,1,1,0x1908,0x1401,rgba)
        return list(rgba)
    for mapped in [0,1]:
        require(render(1,False,mapped)[3]==255,'Single-sided front missing')
        require(render(1,True,mapped)[3]==0,'Single-sided back not culled')
        front=render(2,False,mapped);back=render(2,True,mapped)
        require(front[3]==back[3]==255,'Double-sided coverage missing')
        require(all(abs(a+b-255)<=3 for a,b in zip(front[:3],back[:3])),f'Back normal did not reverse: {front}, {back}')
    # Exercise the production color shader as well, isolating its material
    # sampling/coverage with flat shading instead of relying on a normal pass.
    api(G,'glUseProgram',None,U)(scene)
    for index in range(16):api(G,'glDisableVertexAttribArray',None,U)(index)
    for name,size,offset in [('position',3,0),('normal',3,12),('uv',2,24)]:
        index=api(G,'glGetAttribLocation',I,U,c.c_char_p)(scene,name.encode());require(index>=0,name+' scene attribute')
        api(G,'glEnableVertexAttribArray',None,U)(index)
        api(G,'glVertexAttribPointer',None,U,I,U,U,I,P)(index,size,0x1406,0,32,P(offset))
    color=api(G,'glGetAttribLocation',I,U,c.c_char_p)(scene,b'color')
    require(color>=0,'Color attribute')
    api(G,'glVertexAttrib4f',None,U,F,F,F,F)(color,1,1,1,1)
    api(G,'glUniformMatrix4fv',None,I,I,U,c.POINTER(F))(location(scene,b'view_projection'),1,0,identity)
    api(G,'glUniform2f',None,I,F,F)(location(scene,b'material_size'),1,1)
    def scene_uniform(name,value):api(G,'glUniform1f',None,I,F)(location(scene,name.encode()),value)
    for name,value in [('lit',1),('flat_shading',1),('ambient_strength',1),('has_normal_map',0),('tonemap_here',0)]:
        scene_uniform(name,value)
    api(G,'glUniform1i',None,I,I)(location(scene,b'material'),0)
    alpha_pixel=(c.c_ubyte*4)(255,255,255,64)
    api(G,'glTexSubImage2D',None,U,I,I,I,I,I,U,U,P)(0x0DE1,0,0,0,1,1,0x1908,0x1401,alpha_pixel)
    def color_render(face,back,alpha,cutoff,blend=False):
        scene_uniform('material_face_mode',face);scene_uniform('material_alpha_mode',alpha)
        scene_uniform('material_alpha_cutoff',cutoff)
        api(G,'glFrontFace',None,U)(0x0900 if back else 0x0901)
        api(G,'glEnable' if blend else 'glDisable',None,U)(0x0BE2)
        api(G,'glBlendFunc',None,U,U)(0x0302,0x0303)
        api(G,'glClearColor',None,F,F,F,F)(0,0,0,0);api(G,'glClear',None,U)(0x4000)
        api(G,'glDrawArrays',None,U,I,I)(4,0,3)
        rgba=(c.c_ubyte*4)();api(G,'glReadPixels',None,I,I,I,I,U,U,P)(32,24,1,1,0x1908,0x1401,rgba)
        return list(rgba)
    opaque=color_render(1,False,0,.5)
    require(opaque[0]>100 and opaque[3]==255,'Opaque color lost coverage to texture alpha')
    require(color_render(1,True,0,.5)==[0,0,0,0],'Color single-sided back not culled')
    require(color_render(2,True,0,.5)[3]==255,'Color double-sided back missing')
    require(color_render(2,False,1,.2)[3]==255,'Mask rejected above-cutoff texel')
    require(color_render(2,False,1,.3)==[0,0,0,0],'Mask ignored cutoff')
    require(abs(color_render(2,False,2,.5)[3]-64)<=1,'Blend shader lost continuous alpha')
    # The real surface shader must draw the grid, retain distant major lines,
    # and return pixel-for-pixel to its original output when disabled.
    def grid_image(spacing, opacity, distance):
        api(G,'glUniform4f',None,I,F,F,F,F)(location(scene,b'surface_grid'),spacing,opacity,0,0)
        api(G,'glUniform3f',None,I,F,F,F)(location(scene,b'camera_position'),0,0,distance)
        color_render(2,False,0,.5)
        pixels=(c.c_ubyte*(64*64*4))()
        api(G,'glReadPixels',None,I,I,I,I,U,U,P)(0,0,64,64,0x1908,0x1401,pixels)
        return bytes(pixels)
    without=grid_image(0,0,1)
    near=grid_image(.125,.7,1)
    far=grid_image(.125,.7,2400)
    require(sum(a!=b for a,b in zip(without,near))>100,'Near surface grid missing')
    require(sum(a!=b for a,b in zip(without,far))>10,'Distant grid disappeared')
    require(near!=grid_image(1.0,.7,1),'Grid spacing has no effect')
    require(without==grid_image(.125,0,1),'Disabled grid changes output')
    require(without==grid_image(0,.7,1),'Hidden grid changes output')
    blended=color_render(2,False,2,.5,True)
    require(0<blended[0]<opaque[0]/2,'Blend framebuffer result is not partial coverage')
    api(G,'glDisable',None,U)(0x0BE2)
    # Read actual depth, so a mask hole cannot pass by merely painting black.
    depth=program('MATERIAL_DEPTH');fast_depth=program('DEPTH')
    api(G,'glUseProgram',None,U)(depth)
    for index in range(16):api(G,'glDisableVertexAttribArray',None,U)(index)
    for name,size,offset in [('position',3,0),('uv',2,24)]:
        index=api(G,'glGetAttribLocation',I,U,c.c_char_p)(depth,name.encode());require(index>=0,name+' depth attribute')
        api(G,'glEnableVertexAttribArray',None,U)(index)
        api(G,'glVertexAttribPointer',None,U,I,U,U,I,P)(index,size,0x1406,0,32,P(offset))
    api(G,'glUniformMatrix4fv',None,I,I,U,c.POINTER(F))(location(depth,b'view_projection'),1,0,identity)
    api(G,'glUniform2f',None,I,F,F)(location(depth,b'material_size'),1,1)
    api(G,'glUniform1i',None,I,I)(location(depth,b'material'),0)
    depth_bits=I();api(G,'glGetFramebufferAttachmentParameteriv',None,U,U,U,c.POINTER(I))(0x8D40,0x8D00 if sys.platform == 'darwin' else 0x1801,0x8216,c.byref(depth_bits))
    require(depth_bits.value>=16,'Depth attachment unavailable')
    api(G,'glEnable',None,U)(0x0B71);api(G,'glDepthFunc',None,U)(0x0201)
    api(G,'glDepthMask',None,U)(1);api(G,'glDisable',None,U)(0x0B44)
    def depth_render(face,back,alpha,cutoff):
        for name,value in [('material_face_mode',face),('material_alpha_mode',alpha),('material_alpha_cutoff',cutoff)]:
            api(G,'glUniform1f',None,I,F)(location(depth,name.encode()),value)
        api(G,'glFrontFace',None,U)(0x0900 if back else 0x0901)
        api(G,'glClearDepth',None,c.c_double)(1);api(G,'glClear',None,U)(0x0100)
        api(G,'glDrawArrays',None,U,I,I)(4,0,3)
        value=F();api(G,'glReadPixels',None,I,I,I,I,U,U,P)(32,24,1,1,0x1902,0x1406,c.byref(value))
        return value.value
    require(abs(depth_render(1,False,0,.5)-.5)<.001,'Opaque shadow depth missing')
    require(depth_render(1,True,0,.5)>.999,'Single-sided back casts a shadow')
    require(abs(depth_render(2,True,0,.5)-.5)<.001,'Double-sided back shadow missing')
    require(abs(depth_render(2,False,1,.2)-.5)<.001,'Mask kept texel failed to cast shadow')
    require(depth_render(2,False,1,.3)>.999,'Mask hole wrote shadow depth')
    require(depth_render(2,False,2,.5)>.999,'Blend material wrote opaque shadow depth')
    api(G,'glUseProgram',None,U)(fast_depth)
    api(G,'glUniformMatrix4fv',None,I,I,U,c.POINTER(F))(location(fast_depth,b'view_projection'),1,0,identity)
    position=api(G,'glGetAttribLocation',I,U,c.c_char_p)(fast_depth,b'position')
    api(G,'glEnableVertexAttribArray',None,U)(position)
    api(G,'glVertexAttribPointer',None,U,I,U,U,I,P)(position,3,0x1406,0,32,None)
    api(G,'glClear',None,U)(0x0100);api(G,'glDrawArrays',None,U,I,I)(4,0,3)
    fast_value=F();api(G,'glReadPixels',None,I,I,I,I,U,U,P)(32,24,1,1,0x1902,0x1406,c.byref(fast_value))
    require(abs(fast_value.value-.5)<.001,'Ordinary opaque fast shadow depth missing')
    require(api(G,'glGetError',U)()==0,'OpenGL error')
    print('PASS: surface grid near/far/spacing/off, composite compilation; production scene/normal shaders; front/back culling, normal-map reversal, opaque/mask/blend color, framebuffer blending and masked shadow depth')
finally:
    if sys.platform == 'darwin':
        api(G,'CGLSetCurrentContext',I,P)(None)
        api(G,'CGLDestroyContext',I,P)(context)
    else:
        api(E,'eglMakeCurrent',U,P,P,P,P)(display,None,None,None)
        api(E,'eglDestroyContext',U,P,P)(display,context)
        api(E,'eglDestroySurface',U,P,P)(display,surface)
        api(E,'eglTerminate',U,P)(display)
