    // Project-owned prefab components receive native values, without a dependency
    // on HeronBrushEntity or a prescribed game schema.
    private static object ConvertProperty(object value, System.Type type, Dictionary<string,GameObject> nodes)
    {
        if (typeof(UnityEngine.Object).IsAssignableFrom(type) && value is string reference)
        {
            if (nodes.TryGetValue(reference,out var node))
            {
                if (type == typeof(GameObject)) return node;
                if (type == typeof(Transform)) return node.transform;
                var component = node.GetComponentInChildren(type,true);
                if (component == null) throw new System.InvalidOperationException("Missing referenced component " + type + " on " + reference);
                return component;
            }
            var asset = AssetDatabase.LoadAssetAtPath(reference,type);
            if (asset == null) throw new System.InvalidOperationException("Missing referenced entity/asset " + reference);
            return asset;
        }
        if (value is object[] values)
        {
            float F(int i) => System.Convert.ToSingle(values[i],System.Globalization.CultureInfo.InvariantCulture);
            if (type == typeof(Vector2) && values.Length == 2) return new Vector2(F(0),F(1));
            if (type == typeof(Vector3) && values.Length == 3) return new Vector3(F(0),F(1),F(2));
            if (type == typeof(Vector4) && values.Length == 4) return new Vector4(F(0),F(1),F(2),F(3));
            if (type == typeof(Color) && values.Length == 4) return new Color(F(0),F(1),F(2),F(3));
            if (type == typeof(Quaternion) && values.Length == 4) return new Quaternion(F(0),F(1),F(2),F(3));
            if (type.IsArray)
            {
                var element = type.GetElementType(); var array = System.Array.CreateInstance(element,values.Length);
                for(int i=0;i<values.Length;i++) array.SetValue(ConvertProperty(values[i],element,nodes),i);
                return array;
            }
            if(type.IsGenericType && type.GetGenericTypeDefinition() == typeof(List<>))
            {
                var element=type.GetGenericArguments()[0];var list=(System.Collections.IList)System.Activator.CreateInstance(type);
                foreach(var item in values)list.Add(ConvertProperty(item,element,nodes));return list;
            }
        }
        if(type.IsEnum)return value is string name ? System.Enum.Parse(type,name) : System.Enum.ToObject(type,value);
        return System.Convert.ChangeType(value,type,System.Globalization.CultureInfo.InvariantCulture);
    }

    private static void SetProperty(GameObject node, string name, object value, Dictionary<string,GameObject> nodes)
    {
        string componentName=null;
        bool explicitBinding=name.StartsWith("unity_property_");
        if(explicitBinding)
        {
            var parts=name.Substring("unity_property_".Length).Split(new[]{"__"},System.StringSplitOptions.None);
            if(parts.Length!=2)throw new System.InvalidOperationException("Invalid Unity property binding " + name);
            componentName=parts[0];name=parts[1];
        }
        else if(name.StartsWith("_") || name.StartsWith("unity_"))return;
        bool assigned=false;
        foreach(var component in node.GetComponentsInChildren<MonoBehaviour>(true))
        {
            if(component==null)continue;var type=component.GetType();
            if(componentName!=null && type.Name!=componentName && type.FullName!=componentName)continue;
            var field=type.GetField(name,System.Reflection.BindingFlags.Instance|System.Reflection.BindingFlags.Public|System.Reflection.BindingFlags.NonPublic);
            if(field!=null && !field.IsInitOnly && (field.IsPublic || field.IsDefined(typeof(SerializeField),true) || explicitBinding))
            {
                field.SetValue(component,ConvertProperty(value,field.FieldType,nodes));assigned=true;EditorUtility.SetDirty(component);continue;
            }
            var property=type.GetProperty(name,System.Reflection.BindingFlags.Instance|System.Reflection.BindingFlags.Public);
            if(property!=null && property.CanWrite && property.GetIndexParameters().Length==0)
            {property.SetValue(component,ConvertProperty(value,property.PropertyType,nodes));assigned=true;EditorUtility.SetDirty(component);}
        }
        if(explicitBinding && !assigned)throw new System.InvalidOperationException("Missing Unity component/property " + componentName + "." + name);
    }
