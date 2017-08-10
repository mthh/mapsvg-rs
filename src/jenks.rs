use std::f64;

// Lets fake a 2D array (square shape matrix) with a 1D vector:
struct Matrix<T> {
    values: Vec<T>,
    dim: usize,
}

// ... and implement getter and setter methods using 'unsafe' functions :
impl<T: PartialEq + Clone> Matrix<T> {
    pub fn new(init_value: T, dim: usize) -> Matrix<T> {
        Matrix {
            values: vec![init_value; dim * dim],
            dim: dim,
        }
    }

    #[inline(always)]
    pub fn get(&self, ix: (usize, usize)) -> &T {
        unsafe { self.values.get_unchecked(ix.0 * self.dim + ix.1) }
    }

    #[inline(always)]
    pub fn set(&mut self, ix: (usize, usize), value: T) {
        let mut v = unsafe { self.values.get_unchecked_mut(ix.0 * self.dim + ix.1) };
        *v = value;
    }
}

pub fn get_breaks(values: &mut [f64], nb_class: u32) -> Vec<f64> {
    let k = nb_class as usize;
    let nb_elem: usize = values.len();
    let mut v1 = Matrix::new(1, nb_elem);
    let mut v2 = Matrix::new(f64::MAX, nb_elem);

    values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let (mut v, mut val, mut s1, mut s2, mut w, mut i3, mut i4): (f64,
                                                                  f64,
                                                                  f64,
                                                                  f64,
                                                                  f64,
                                                                  usize,
                                                                  usize);

    for l in 2..(nb_elem + 1) {
        s1 = 0.0;
        s2 = 0.0;
        w = 0.0;
        for m in 1..(l + 1) {
            i3 = l - m + 1;
            val = unsafe { *values.get_unchecked(i3 - 1) };
            s2 += val * val;
            s1 += val;
            w += 1.0;
            v = s2 - (s1 * s1) / w;
            i4 = i3 - 1;
            if i4 != 0 {
                for j in 2..k + 1 {
                    let _v = v + v2.get((i4 - 1, j - 2));
                    if *v2.get((l - 1, j - 1)) >= _v {
                        v2.set((l - 1, j - 1), _v);
                        v1.set((l - 1, j - 1), i3);
                    }
                }
            }
            v1.set((l - 1, 0), 1);
            v2.set((l - 1, 0), v);
        }
    }
    let mut kclass = vec![0; k as usize];
    let mut k = nb_elem as u32;
    let mut j = nb_class;
    while j > 1 {
        k = *v1.get(((k - 1) as usize, (j - 1) as usize)) as u32 - 1;
        kclass[(j - 2) as usize] = k;
        j -= 1;
    }
    let mut breaks = Vec::with_capacity(nb_class as usize);
    breaks.push(values[0]);
    for i in 1..nb_class {
        breaks.push(values[(kclass[(i - 1) as usize] - 1) as usize]);
    }
    breaks.push(values[(nb_elem - 1)]);
    breaks
}
