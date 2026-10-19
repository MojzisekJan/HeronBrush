        private static JToken Property(JObject data, string name) => data["properties"]?[name]?["value"];
        private static Type ScriptType(string name)
        {
            foreach(var assembly in AppDomain.CurrentDomain.GetAssemblies())
            {
                var type=assembly.GetType(name);
                if(type!=null && typeof(Script).IsAssignableFrom(type) && !type.IsAbstract)return type;
            }
            throw new InvalidOperationException("Missing project script type " + name);
        }
        private static IEnumerable<Script> ProjectScripts(Actor actor)
        {
            foreach(var script in actor.Scripts)if(script!=null)yield return script;
            foreach(var child in actor.Children)foreach(var script in ProjectScripts(child))yield return script;
        }
        private static JToken Plain(JToken value)
        {
            if(value is JArray array){var result=new JArray();foreach(var item in array)result.Add(Plain(item));return result;}
            if(value is JObject obj && obj["value"]!=null)return Plain(obj["value"]);
            return value.DeepClone();
        }
        private static object ConvertProperty(JToken value,Type type,IReadOnlyDictionary<string,Actor> actors)
        {
            if(typeof(Actor).IsAssignableFrom(type) || typeof(Script).IsAssignableFrom(type))
            {
                if(!actors.TryGetValue((string)value,out var actor))throw new InvalidOperationException("Missing referenced actor " + value);
                if(type.IsInstanceOfType(actor))return actor;
                foreach(var script in ProjectScripts(actor))if(type.IsInstanceOfType(script))return script;
                throw new InvalidOperationException("Missing referenced component " + type + " on " + value);
            }
            if(value is JArray values)
            {
                float F(int i)=>(float)values[i];
                if(type==typeof(Vector2)&&values.Count==2)return new Vector2(F(0),F(1));
                if(type==typeof(Vector3)&&values.Count==3)return new Vector3(F(0),F(1),F(2));
                if(type==typeof(Vector4)&&values.Count==4)return new Vector4(F(0),F(1),F(2),F(3));
                if(type==typeof(Color)&&values.Count==4)return new Color(F(0),F(1),F(2),F(3));
                if(type==typeof(Quaternion)&&values.Count==4)return new Quaternion(F(0),F(1),F(2),F(3));
            }
            return value.ToObject(type);
        }
        private static void ConfigureProperties(Actor actor,JObject data,IReadOnlyDictionary<string,Actor> actors)
        {
            if(!(data["properties"] is JObject properties))return;
            foreach(var property in properties.Properties())
            {
                var name=property.Name;string componentName=null;bool strict=name.StartsWith("flax_property_");
                if(strict){var parts=name.Substring("flax_property_".Length).Split(new[]{"__"},StringSplitOptions.None);if(parts.Length!=2)throw new InvalidOperationException("Invalid Flax property " + name);componentName=parts[0];name=parts[1];}
                else if(name.StartsWith("_")||name.StartsWith("flax_"))continue;
                var value=Plain(property.Value);bool assigned=false;
                foreach(var script in ProjectScripts(actor))
                {
                    var type=script.GetType();if(componentName!=null&&componentName!=type.Name&&componentName!=type.FullName)continue;
                    var field=type.GetField(name,System.Reflection.BindingFlags.Instance|System.Reflection.BindingFlags.Public|System.Reflection.BindingFlags.NonPublic);
                    if(field!=null&&!field.IsInitOnly&&(field.IsPublic||strict)){field.SetValue(script,ConvertProperty(value,field.FieldType,actors));assigned=true;continue;}
                    var target=type.GetProperty(name,System.Reflection.BindingFlags.Instance|System.Reflection.BindingFlags.Public);
                    if(target!=null&&target.CanWrite&&target.GetIndexParameters().Length==0){target.SetValue(script,ConvertProperty(value,target.PropertyType,actors));assigned=true;}
                }
                if(strict&&!assigned)throw new InvalidOperationException("Missing Flax script/property " + componentName + "." + name);
            }
        }
