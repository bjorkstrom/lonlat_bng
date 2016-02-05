// mod shifts;
// use shifts::get_shifts_hashmap;
use super::PI;
use super::AIRY_1830_SEMI_MAJOR;
use super::AIRY_1830_SEMI_MINOR;
use super::GRS80_SEMI_MAJOR;
use super::GRS80_SEMI_MINOR;

const RAD: f64 = PI / 180.;
const WGS84_A: f64 = GRS80_SEMI_MAJOR;
const WGS84_B: f64 = GRS80_SEMI_MINOR;

// lon and lat of true origin
const LAM0: f64 = RAD * -2.0;
const PHI0: f64 = RAD * 49.0;

// Easting and Northing of origin
const E0: f64 = 400000.;
const N0: f64 = -100000.;
// convergence factor
const F0: f64 = 0.9996012717;

const MIN_X_SHIFT: f64 = 86.275;
const MIN_Y_SHIFT: f64 = -81.603;
const MIN_Z_SHIFT: f64 = 43.982;

use std::collections::HashMap;

// Herbie's going to have a field day with this
pub fn round_to_nearest_mm(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let new_x = (x * 1000.).round() as f64 / 1000.;
    let new_y = (y * 1000.).round() as f64 / 1000.;
    let new_z = (z * 1000.).round() as f64 / 1000.;
    (new_x, new_y, new_z)
}

pub fn get_ostn_ref(x: &i32, y: &i32) -> (f64, f64, f64) {
    // TODO populate ostn02 with the full OSTN02 data
    // let shifts = get_shifts_hashmap();
    let mut keys = vec!["13928b", "13928c", "13a28b", "13a28c"];
    let mut values: Vec<(_, _, _)> = vec![(16500, 3359, 270),
                                          (16538, 3357, 254),
                                          (16508, 3387, 258),
                                          (16547, 3376, 242)];
    let ostn02 = keys.drain(..).zip(values.drain(..)).collect::<HashMap<_, (_, _, _)>>();
    let key = format!("{:03x}{:03x}", y, x);
    // some or None, so try! this
    let result = ostn02.get(&*key).unwrap();
    // if we get a hit
    let data2 = (result.0 as f64 / 1000. + MIN_X_SHIFT,
                 result.1 as f64 / 1000. + MIN_Y_SHIFT,
                 result.2 as f64 / 1000. + MIN_Z_SHIFT);
    data2
}

// Input values must be valid ETRS89 grid references
// See p20 of the transformation user guide at
// https://www.ordnancesurvey.co.uk/business-and-government/help-and-support/navigation-technology/os-net/formats-for-developers.html
pub fn ostn02_shifts(x: &f64, y: &f64) -> (f64, f64, f64) {
    let e_index = (*x / 1000.) as i32;
    let n_index = (*y / 1000.) as i32;

    // eastings and northings of the south-west corner of the cell
    let x0 = e_index * 1000;
    let y0 = n_index * 1000;

    // The easting, northing and geoid shifts for the four corners of the cell
    // any of these could be Err, so use try!
    let s0_ref: (f64, f64, f64) = get_ostn_ref(&(e_index + 0), &(n_index + 0));
    let s1_ref: (f64, f64, f64) = get_ostn_ref(&(e_index + 1), &(n_index + 0));
    let s2_ref: (f64, f64, f64) = get_ostn_ref(&(e_index + 0), &(n_index + 1));
    let s3_ref: (f64, f64, f64) = get_ostn_ref(&(e_index + 1), &(n_index + 1));
    // only continue if we get Results for the above

    // offset within square
    let dx = *x - (x0 as f64);
    let dy = *y - (y0 as f64);

    // the python script divides by an int here, which = 0 (e.g. 300 / 1000)
    let t = dx / 1000.;
    let u = dy / 1000.;

    let f0 = (1. - t as f64) * (1. - u as f64);
    let f1 = t as f64 * (1. - u as f64);
    let f2 = (1. - t as f64) * u as f64;
    let f3 = t as f64 * u as f64;

    // bilinear interpolation, to obtain the actual shifts
    let se = f0 * s0_ref.0 + f1 * s1_ref.0 + f2 * s2_ref.0 + f3 * s3_ref.0;
    let sn = f0 * s0_ref.1 + f1 * s1_ref.1 + f2 * s2_ref.1 + f3 * s3_ref.1;
    let sg = f0 * s0_ref.2 + f1 * s1_ref.2 + f2 * s2_ref.2 + f3 * s3_ref.2;

    let (r_se, r_sn, r_sg) = round_to_nearest_mm(se, sn, sg);
    (r_se, r_sn, r_sg)

}

// See Annexe B (p23) of the transformation user guide for instructions
pub fn convert_etrs89(longitude: &f64, latitude: &f64) -> (f64, f64) {
    let alt = 0.0;
    // ellipsoid squared eccentricity constant
    let e2 = (WGS84_A.powf(2.) - WGS84_B.powf(2.)) / WGS84_A.powf(2.);
    let n = (WGS84_A - WGS84_B) / (WGS84_A + WGS84_B);
    let phi = RAD * *latitude;
    let lambda = RAD * *longitude;

    let sp2 = phi.sin().powf(2.);
    let nu = WGS84_A * F0 * (1. - e2 * sp2).powf(-0.5); // v
    let rho = WGS84_A * F0 * (1. - e2) * (1. - e2 * sp2).powf(-1.5);
    let eta2 = nu / rho - 1.;

    let m = compute_m(&phi, &WGS84_B, &n);

    let cp = phi.cos();
    let sp = phi.sin();
    let tp = phi.tan();
    let tp2 = tp.powf(2.);
    let tp4 = tp.powf(4.);

    let I = m + N0;
    let II = nu / 2. * sp * cp;
    let III = nu / 24. * sp * cp.powf(3.) * (5. - tp2 + 9. * eta2);
    let IIIA = nu / 720. * sp * cp.powf(5.) * (61. - 58. * tp2 + tp4);

    let IV = nu * cp;
    let V = nu / 6. * cp.powf(3.) * (nu / rho - tp2);
    let VI = nu / 120. * cp.powf(5.) * (5. - 18. * tp2 + tp4 + 14. * eta2 - 58. * tp2 * eta2);

    let l = lambda - LAM0;
    let north = I + II * l.powf(2.) + III * l.powf(4.) + IIIA * l.powf(6.);
    let east = E0 + IV * l + V * l.powf(3.) + VI * l.powf(5.);
    let (rounded_eastings, rounded_northings, _) = round_to_nearest_mm(east, north, 1.00);
    (rounded_eastings, rounded_northings)
}

fn compute_m(phi: &f64, b: &f64, n: &f64) -> f64 {
    let p_plus = *phi + PHI0;
    let p_minus = *phi - PHI0;

    let result = *b * F0 *
                 ((1. + *n * (1. + 5. / 4. * *n * (1. + *n))) * p_minus -
                  3. * *n * (1. + *n * (1. + 7. / 8. * *n)) * p_minus.sin() * p_plus.cos() +
                  (15. / 8. * *n * (*n * (1. + *n))) * (2. * p_minus).sin() * (2. * p_plus).cos() -
                  35. / 24. * n.powf(3.) * (3. * p_minus).sin() * (3. * p_plus).cos());
    result
}
// Convert OSGB36 coordinates to ETRS89 using OSTN02 shifts
// pub fn shift_osgb36_to_etrs89(E: &f64, N: &f64) -> (f64, f64) {
//     let z0 = 0.000;
//     let epsilon = 0.00001;
//     let (dx, dy, dz) = ostn02_shifts(&E, &N);
//     let (x, y, z) = (E - dx, N - dy, dz);
//     let (last_dx, last_dy) = (dx.clone(), dy.clone());
//     while (dx - last_dx).abs() < epsilon && (dy - last_dy).abs() < epsilon {
//         let (dx, dy, dz) = ostn02_shifts(&x, &y);
//         let (x, y) = (E - dx, N - dy);
//         let (last_dx, last_dy) = (dx, dy);
//     }
//     let (x, y, z) = round_to_nearest_mm(E - dx, N - dy, z0 + dz);
//     (x, y)
// }
