using UnityEngine;
// Entirely project-owned; no generated runtime or JSON dependency.
public sealed class Puzzle : MonoBehaviour {
    public int charge;
    public bool armed;
    public int[] sequence;
    public Puzzle partner;
    public void Activate(){if(armed&&partner!=null)partner.charge++;}
}
