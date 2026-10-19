using System;
using System.Collections;
using UnityEngine;
namespace HeronBrush.Demo {
    public class HeronDemoCaptureProbe : MonoBehaviour {
        string path;
        [RuntimeInitializeOnLoadMethod]
        static void Setup() {
            var args=Environment.GetCommandLineArgs();int index=Array.IndexOf(args,"--heron-demo-capture");
            if(index<0||index+1>=args.Length)return;
            new GameObject("Capture probe").AddComponent<HeronDemoCaptureProbe>().path=args[index+1];
        }
        IEnumerator Start() {
            yield return new WaitForSeconds(2);
            yield return new WaitForEndOfFrame();
            ScreenCapture.CaptureScreenshot(path);
            yield return new WaitForSeconds(1);
            Debug.Log("HERON_GAMEPLAY_CAPTURE PASS");Application.Quit(0);
        }
    }
}
