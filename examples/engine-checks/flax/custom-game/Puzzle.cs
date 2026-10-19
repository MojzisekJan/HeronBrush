using FlaxEngine;
namespace Game {
    public sealed class Puzzle : Script {
        public int charge;
        public bool armed;
        public int[] sequence;
        public Puzzle partner;
        public void Activate(){if(armed&&partner!=null)partner.charge++;}
    }
}
