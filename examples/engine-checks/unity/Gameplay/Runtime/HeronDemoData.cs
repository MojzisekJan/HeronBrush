using System;
using System.Globalization;
using System.Linq;
using UnityEngine;
namespace HeronBrush.Demo {
    [Serializable] public class ColorValue { public float[] value; }
    [Serializable] public class Property { public string name, value; }
    [Serializable] public class EntityData {
        public string name, classname;
        public string[] targets;
        public Property[] properties;
        public string Text(string key, string fallback="") => properties?.FirstOrDefault(p=>p.name==key)?.value ?? fallback;
        public float Number(string key, float fallback=0) => float.TryParse(Text(key),NumberStyles.Float,CultureInfo.InvariantCulture,out var n) ? n : fallback;
        public Color Tint(Color fallback) {
            try {
                var color=JsonUtility.FromJson<ColorValue>("{\"value\":"+Text("color","[]")+"}").value;
                return color!=null&&color.Length>=3?new Color(color[0],color[1],color[2],color.Length>3?color[3]:1):fallback;
            } catch(ArgumentException) {return fallback;}
        }
        public Vector3 Direction(string key, string fallback="0 0 1") {
            var n=Text(key,fallback).Split(new[]{' '},StringSplitOptions.RemoveEmptyEntries);
            if(n.Length!=3) return Vector3.up;
            float P(int i)=>float.TryParse(n[i],NumberStyles.Float,CultureInfo.InvariantCulture,out var v)?v:0;
            // Map X/Y/Z become Unity -Y/Z/X.
            var v=new Vector3(-P(1),P(2),P(0));
            return v.sqrMagnitude>0?v.normalized:Vector3.up;
        }
    }
    [Serializable] public class LevelData { public int version; public float scale=32; public EntityData[] entities; }
}
