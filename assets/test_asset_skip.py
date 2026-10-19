"""Run with: python3 -m unittest discover -s assets -p test_asset_skip.py"""
import unittest
import numpy as np
from asset_skip import area, subtract


def box(x0,x1,y0=-1,y1=5,z0=-1,z1=1):
    return [(np.array(n,dtype=float),d) for n,d in
            [((-1,0,0),-x0),((1,0,0),x1),((0,-1,0),-y0),((0,1,0),y1),((0,0,-1),-z0),((0,0,1),z1)]]


def rectangle(x0,x1):
    return [np.array(v,dtype=float) for v in [(x0,0,0),(x1,0,0),(x1,4,0),(x0,4,0)]]


class CoverageTests(unittest.TestCase):
    def test_touching_edge_does_not_erase_polygon(self):
        self.assertAlmostEqual(sum(area(p) for p in subtract(rectangle(2,4),box(0,2))),8)

    def test_partial_cover_keeps_exposed_part(self):
        self.assertAlmostEqual(sum(area(p) for p in subtract(rectangle(0,4),box(0,2))),8)

    def test_union_can_cover_entire_face(self):
        pieces=subtract(rectangle(0,4),box(0,2))
        pieces=[p for part in pieces for p in subtract(part,box(2,4))]
        self.assertEqual(pieces,[])

    def test_gap_between_covering_brushes_remains_visible(self):
        pieces=subtract(rectangle(0,4),box(0,1.9))
        pieces=[p for part in pieces for p in subtract(part,box(2.1,4))]
        self.assertAlmostEqual(sum(area(p) for p in pieces),.8)

    def test_fully_inside_is_removed(self):
        self.assertEqual(subtract(rectangle(1,3),box(0,4)),[])




def solid_box(bid,lo,hi,material='stone'):
    import itertools
    faces=[]
    for axis in range(3):
        other=[i for i in range(3) if i!=axis]
        for sign in (-1,1):
            normal=[0.,0.,0.];normal[axis]=float(sign)
            vertices=[]
            for u,v in ((0,0),(1,0),(1,1),(0,1)):
                p=[0.,0.,0.];p[axis]=(lo if sign<0 else hi)[axis]
                p[other[0]]=(lo,hi)[u][other[0]];p[other[1]]=(lo,hi)[v][other[1]]
                vertices.append(p)
            faces.append({'face':len(faces),'normal':normal,'vertices':vertices,'material':material})
    return {'id':str(bid),'min':lo,'max':hi,'faces':faces}


class SharedFaceTests(unittest.TestCase):
    def test_identical_contact_marks_both_sides_only(self):
        from asset_skip import hidden_faces
        a=solid_box(1,[0,0,0],[1,1,1]);b=solid_box(2,[1,0,0],[2,1,1])
        self.assertEqual(set(hidden_faces([a,b])),{('1',1),('2',0)})

    def test_tiled_contact_covers_the_large_face(self):
        from asset_skip import hidden_faces
        a=solid_box(1,[0,0,0],[1,2,1]);b=solid_box(2,[1,0,0],[2,1,1]);c=solid_box(3,[1,1,0],[2,2,1])
        hidden=set(hidden_faces([a,b,c]))
        self.assertTrue({('1',1),('2',0),('3',0)}<=hidden)

    def test_partial_contact_keeps_large_face(self):
        from asset_skip import hidden_faces
        a=solid_box(1,[0,0,0],[1,2,1]);b=solid_box(2,[1,0,0],[2,1,1])
        self.assertEqual(set(hidden_faces([a,b])),{('2',0)})

    def test_separate_parallel_faces_stay_visible(self):
        from asset_skip import hidden_faces
        a=solid_box(1,[0,0,0],[1,1,1]);b=solid_box(2,[1.0001,0,0],[2,1,1])
        self.assertEqual(hidden_faces([a,b]),[])

    def test_edge_contact_is_not_face_coverage(self):
        from asset_skip import hidden_faces
        a=solid_box(1,[0,0,0],[1,1,1]);b=solid_box(2,[1,1,0],[2,2,1])
        self.assertEqual(hidden_faces([a,b]),[])

    def test_glass_does_not_hide_opaque_neighbour(self):
        from asset_skip import hidden_faces
        a=solid_box(1,[0,0,0],[1,1,1]);b=solid_box(2,[1,0,0],[2,1,1],'glass')
        self.assertNotIn(('1',1),hidden_faces([a,b]))


    def test_coplanar_cover_with_a_gap_keeps_large_face(self):
        from asset_skip import hidden_faces
        a=solid_box(1,[0,0,0],[1,2,1]);b=solid_box(2,[1,0,0],[2,.9,1]);c=solid_box(3,[1,1.1,0],[2,2,1])
        self.assertNotIn(('1',1),hidden_faces([a,b,c]))

    def test_rotated_shared_faces(self):
        from asset_skip import hidden_faces
        rotation=np.array([[.6,-.8,0],[.8,.6,0],[0,0,1]])
        brushes=[solid_box(1,[0,0,0],[1,1,1]),solid_box(2,[1,0,0],[2,1,1])]
        for brush in brushes:
            for face in brush['faces']:
                face['normal']=(rotation@face['normal']).tolist()
                face['vertices']=[(rotation@v).tolist() for v in face['vertices']]
            vertices=np.array([v for f in brush['faces'] for v in f['vertices']])
            brush['min']=vertices.min(axis=0).tolist();brush['max']=vertices.max(axis=0).tolist()
        self.assertEqual(set(hidden_faces(brushes)),{('1',1),('2',0)})

    def test_duplicate_exterior_faces_are_not_internal(self):
        from asset_skip import hidden_faces
        self.assertEqual(hidden_faces([solid_box(1,[0,0,0],[1,1,1]),solid_box(2,[0,0,0],[1,1,1])]),[])


if __name__=='__main__': unittest.main()
