// What the page takes of three.js (MIT, `ui/vendor/three-LICENSE.txt`): the 3D bus viewer's
// scene, lights, loader and camera controls, bundled into `ui/vendor/three.js`:
//
//   npm pack three@0.186.1 && tar -xzf three-0.186.1.tgz && mkdir -p node_modules && mv package node_modules/three
//   npx esbuild tools/three-entry.js --bundle --format=esm --minify --legal-comments=none --outfile=ui/vendor/three.js

export { Scene, PerspectiveCamera, WebGLRenderer, AmbientLight, HemisphereLight, DirectionalLight, Box3, Vector3, Color, Mesh, MeshStandardMaterial, CircleGeometry, PlaneGeometry, ShadowMaterial, PCFSoftShadowMap, SRGBColorSpace, ACESFilmicToneMapping, Group, Fog } from 'three';
export { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
export { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
